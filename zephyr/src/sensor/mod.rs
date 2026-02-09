//! Generic sensor driver framework
//!
//! Provides traits and types for building Zephyr sensor drivers in Rust.

pub mod types;
pub use types::*;

use crate::device::DeviceRef;

/// Trait that all sensor drivers must implement
///
/// This trait provides type-safe access to driver data and configuration
/// through associated types. The macro `sensor_ffi_exports!` generates the
/// necessary FFI glue code that automatically extracts typed references.
///
/// # Associated Types
/// * `Data` - The C-compatible data struct (must have a `rust_ptr: *mut Self::DataInner` field)
/// * `DataInner` - The actual Rust data structure allocated on the heap
/// * `Config` - The C-compatible config struct
///
/// # Example
/// ```rust,no_run
/// use zephyr::sensor::{SensorDriver, SensorChannel, SensorResult, SensorValue};
/// use zephyr::device::DeviceRef;
///
/// struct MyDriver;
///
/// impl SensorDriver for MyDriver {
///     type Data = MyData;           // C struct with rust_ptr
///     type DataInner = MyDataInner; // Rust struct on heap
///     type Config = MyConfig;       // C config struct
///
///     fn init(&mut self, dev: DeviceRef, data: &mut Self::Data, config: &Self::Config) -> SensorResult<()> {
///         // Allocate DataInner and store pointer in data.rust_ptr
///         Ok(())
///     }
///
///     fn sample_fetch(
///         &mut self,
///         dev: DeviceRef,
///         data: &mut Self::DataInner,
///         config: &Self::Config,
///         channel: SensorChannel,
///     ) -> SensorResult<()> {
///         // No unsafe needed - data and config are already typed!
///         Ok(())
///     }
///
///     fn channel_get(
///         &self,
///         dev: DeviceRef,
///         data: &Self::DataInner,
///         config: &Self::Config,
///         channel: SensorChannel,
///     ) -> SensorResult<SensorValue> {
///         Ok(SensorValue::default())
///     }
/// }
/// ```
pub trait SensorDriver {
    /// The C-compatible data struct that holds a pointer to the Rust data
    /// Must have a `rust_ptr: *mut Self::DataInner` field
    type Data;
    /// The actual Rust data structure allocated on the heap
    type DataInner;
    /// The C-compatible config structure
    type Config;

    /// Initialize the driver
    ///
    /// This method handles allocation and setup. It receives:
    /// - `dev` - Safe reference to the device structure
    /// - `data` - Mutable reference to the C data struct (for storing rust_ptr)
    /// - `config` - Reference to the config structure
    fn init(
        &mut self,
        dev: DeviceRef,
        data: &mut Self::Data,
        config: &Self::Config,
    ) -> SensorResult<()>;

    /// Fetch sample(s) from the sensor
    ///
    /// # Parameters
    /// * `dev` - Safe reference to the device structure
    /// * `data` - Mutable reference to the driver's data structure
    /// * `config` - Reference to the driver's config structure
    /// * `channel` - The sensor channel to fetch from
    fn sample_fetch(
        &mut self,
        dev: DeviceRef,
        data: &mut Self::DataInner,
        config: &Self::Config,
        channel: SensorChannel,
    ) -> SensorResult<()>;

    /// Get channel value (after fetch)
    ///
    /// # Parameters
    /// * `dev` - Safe reference to the device structure
    /// * `data` - Reference to the driver's data structure
    /// * `config` - Reference to the driver's config structure
    /// * `channel` - The sensor channel to read from
    fn channel_get(
        &self,
        dev: DeviceRef,
        data: &Self::DataInner,
        config: &Self::Config,
        channel: SensorChannel,
    ) -> SensorResult<SensorValue>;

    /// Set sensor attribute (optional)
    ///
    /// # Parameters
    /// * `_dev` - Safe reference to the device structure
    /// * `_data` - Mutable reference to the driver's data structure
    /// * `_config` - Reference to the driver's config structure
    /// * `_channel` - The sensor channel
    /// * `_attr` - The attribute ID to set
    /// * `_value` - The value to set
    fn attr_set(
        &mut self,
        _dev: DeviceRef,
        _data: &mut Self::DataInner,
        _config: &Self::Config,
        _channel: SensorChannel,
        _attr: i32,
        _value: &SensorValue,
    ) -> SensorResult<()> {
        Err(SensorError::NotSupported)
    }

    /// Get sensor attribute (optional)
    ///
    /// # Parameters
    /// * `_dev` - Safe reference to the device structure
    /// * `_data` - Reference to the driver's data structure
    /// * `_config` - Reference to the driver's config structure
    /// * `_channel` - The sensor channel
    /// * `_attr` - The attribute ID to get
    fn attr_get(
        &self,
        _dev: DeviceRef,
        _data: &Self::DataInner,
        _config: &Self::Config,
        _channel: SensorChannel,
        _attr: i32,
    ) -> SensorResult<SensorValue> {
        Err(SensorError::NotSupported)
    }

    /// Set trigger configuration (optional)
    ///
    /// # Parameters
    /// * `_dev` - Safe reference to the device structure
    /// * `_data` - Mutable reference to the driver's data structure
    /// * `_config` - Reference to the driver's config structure
    /// * `_handler` - Trigger handler callback
    fn trigger_set(
        &mut self,
        _dev: DeviceRef,
        _data: &mut Self::DataInner,
        _config: &Self::Config,
        _handler: crate::raw::sensor_trigger_handler_t,
    ) -> SensorResult<()> {
        Err(SensorError::NotSupported)
    }
}

/// Macro to generate FFI exports for a sensor driver
///
/// This macro generates all the necessary FFI glue code to export a Rust
/// sensor driver to C code. It creates:
/// - A static driver_api vtable
/// - FFI wrapper functions for all sensor operations
/// - Helper functions to extract typed data and config references
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
/// use zephyr::sensor::{SensorDriver, SensorChannel, SensorResult, SensorValue};
/// use zephyr::device::DeviceRef;
///
/// struct MyDriver;
/// impl MyDriver { const fn new() -> Self { Self } }
///
/// impl SensorDriver for MyDriver {
///     type Data = MyData;
///     type DataInner = MyDataInner;
///     type Config = MyConfig;
///
///     fn init(&mut self, dev: DeviceRef, data: &mut Self::Data, config: &Self::Config) -> SensorResult<()> { Ok(()) }
///     fn sample_fetch(&mut self, dev: DeviceRef, data: &mut Self::DataInner, config: &Self::Config, ch: SensorChannel) -> SensorResult<()> { Ok(()) }
///     fn channel_get(&self, dev: DeviceRef, data: &Self::DataInner, config: &Self::Config, ch: SensorChannel) -> SensorResult<SensorValue> { Ok(SensorValue::default()) }
/// }
///
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
///
/// # Safety
///
/// The generated code properly handles the FFI boundary and converts between
/// Rust and C types. Raw pointers from C are validated and converted to safe
/// `DeviceRef` wrappers at the FFI boundary. The driver instance is stored in
/// a static mutable variable and must be initialized with a `const fn new()` constructor.
#[macro_export]
macro_rules! sensor_ffi_exports {
    (
        driver: $driver_type:ty,
        prefix: $prefix:ident
    ) => {
        use core::ffi::c_int;
        use $crate::device::DeviceRef;
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

            /// Helper to extract typed data reference
            #[inline]
            unsafe fn [<__ $prefix _get_data>](dev_ref: &DeviceRef) -> &'static mut <$driver_type as SensorDriver>::DataInner {
                let data_ptr = dev_ref.data_as_mut::<<$driver_type as SensorDriver>::Data>();
                &mut *data_ptr.rust_ptr
            }

            /// Helper to extract typed config reference
            #[inline]
            unsafe fn [<__ $prefix _get_config>](dev_ref: &DeviceRef) -> &'static <$driver_type as SensorDriver>::Config {
                dev_ref.config_as::<<$driver_type as SensorDriver>::Config>()
            }

            /// Init function
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn [<$prefix _init>](
                dev: *const $crate::raw::device
            ) -> c_int {
                // SAFETY: The FFI bindings to set logger is safe to call here.
                unsafe {
                    zephyr::set_logger().unwrap();
                }

                // SAFETY: Convert raw pointer to safe DeviceRef at FFI boundary
                let dev_ref = match unsafe { DeviceRef::from_ptr(dev) } {
                    Some(d) => d,
                    None => return -22, // -EINVAL
                };

                let data = dev_ref.data_as_mut::<<$driver_type as SensorDriver>::Data>();
                let config = unsafe { [<__ $prefix _get_config>](&dev_ref) };

                match DRIVER_INSTANCE.init(dev_ref, data, config) {
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
                let dev_ref = match unsafe { DeviceRef::from_ptr(dev) } {
                    Some(d) => d,
                    None => return -22,
                };

                let data = unsafe { [<__ $prefix _get_data>](&dev_ref) };
                let config = unsafe { [<__ $prefix _get_config>](&dev_ref) };
                let value = SensorValue::new((*val).val1, (*val).val2);

                match DRIVER_INSTANCE.attr_set(
                    dev_ref,
                    data,
                    config,
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
                let dev_ref = match unsafe { DeviceRef::from_ptr(dev) } {
                    Some(d) => d,
                    None => return -22,
                };

                let data = unsafe { [<__ $prefix _get_data>](&dev_ref) };
                let config = unsafe { [<__ $prefix _get_config>](&dev_ref) };

                match DRIVER_INSTANCE.attr_get(
                    dev_ref,
                    data,
                    config,
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
                let dev_ref = match unsafe { DeviceRef::from_ptr(dev) } {
                    Some(d) => d,
                    None => return -22,
                };

                if trig.is_null() || handler.is_none() {
                    return -22;
                }

                let data = unsafe { [<__ $prefix _get_data>](&dev_ref) };
                let config = unsafe { [<__ $prefix _get_config>](&dev_ref) };

                match DRIVER_INSTANCE.trigger_set(dev_ref, data, config, handler) {
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
                let dev_ref = match unsafe { DeviceRef::from_ptr(dev) } {
                    Some(d) => d,
                    None => return -22,
                };

                let data = unsafe { [<__ $prefix _get_data>](&dev_ref) };
                let config = unsafe { [<__ $prefix _get_config>](&dev_ref) };

                match DRIVER_INSTANCE.sample_fetch(dev_ref, data, config, SensorChannel::from(chan as i32)) {
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
                let dev_ref = match unsafe { DeviceRef::from_ptr(dev) } {
                    Some(d) => d,
                    None => return -22,
                };

                let data = unsafe { [<__ $prefix _get_data>](&dev_ref) };
                let config = unsafe { [<__ $prefix _get_config>](&dev_ref) };

                match DRIVER_INSTANCE.channel_get(dev_ref, data, config, SensorChannel::from(chan as i32)) {
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
