#![no_std]
#![no_main]

//! TMP108 Temperature Sensor Driver
//!
//! This driver provides support for Texas Instruments TMP108 series
//! temperature sensors using Zephyr's generic sensor framework in Rust.

extern crate alloc;

use alloc::boxed::Box;
use core::cell::{Cell, UnsafeCell};
use core::mem;
use log::info;
use tmp108::{Thermostat, Tmp108};
use zephyr::gpio::GpioPin;
use zephyr::i2c::I2cDevice;
use zephyr::raw;
use zephyr::sensor::{SensorError, SensorResult};

/// FFI binding for tmp108_rs_config struct from C code.
///
/// This must match the C struct definition exactly.
#[repr(C)]
pub struct Tmp108RsConfig {
    /// I2C device tree spec
    pub bus: zephyr::raw::i2c_dt_spec,
    /// GPIO alert pin spec (conditionally compiled with CONFIG_TMP108_RS_ALERT_INTERRUPTS)
    pub alert_gpio: zephyr::raw::gpio_dt_spec,
}

/// C struct that holds a pointer to the Rust data.
/// This must match the C struct definition exactly.
#[repr(C)]
struct Tmp108RsDataPtr {
    rust_ptr: *mut Tmp108RsData,
}

/// Storage for GPIO interrupt callback with device context
///
/// This structure wraps the Zephyr gpio_callback and stores a pointer
/// to the device, allowing the interrupt handler to access Tmp108RsData.
struct Tmp108CallbackStorage {
    /// Zephyr callback structure
    callback: UnsafeCell<raw::gpio_callback>,
    /// Pointer to the device for accessing Tmp108RsData
    device_ptr: *const raw::device,
}

impl Tmp108CallbackStorage {
    /// Create a new callback storage
    fn new(device_ptr: *const raw::device) -> Self {
        Self {
            callback: UnsafeCell::new(unsafe { mem::zeroed() }),
            device_ptr,
        }
    }

    /// C callback handler that retrieves device context and calls trigger handler
    extern "C" fn callback_handler(
        _gpio: *const raw::device,
        cb: *mut raw::gpio_callback,
        _pins: raw::gpio_port_pins_t,
    ) {
        // Logging acquire interrups, so commenting here for now till an alternative is found
        //info!("TMP108 GPIO interrupt fired, pins: 0x{:08X}", pins);

        // TODO: Figure out a cleaner way to do this.
        // Use pointer arithmetic to get back to the Tmp108CallbackStorage
        let storage = unsafe {
            cb.cast::<u8>()
                .sub(mem::offset_of!(Self, callback))
                .cast::<Self>()
        };

        unsafe {
            let device_ptr = (*storage).device_ptr;

            // Get the data pointer from the device
            let data_ptr_struct = (*device_ptr).data.cast::<Tmp108RsDataPtr>();
            let data = &mut *(*data_ptr_struct).rust_ptr;

            // Call the registered trigger handler if set
            if let Some(handler) = data.trigger_handler.get() {
                let trigger_dev = data.trigger_device.get();

                // Create a sensor_trigger for SENSOR_TRIG_DATA_READY
                let trigger = raw::sensor_trigger {
                    type_: raw::sensor_trigger_type_SENSOR_TRIG_DATA_READY,
                    chan: raw::sensor_channel_SENSOR_CHAN_ALL,
                };

                //info!("Calling trigger handler");
                handler(trigger_dev, &trigger);
            } else {
                //info!("No trigger handler registered");
            }
        }
    }
}

// SAFETY: Tmp108CallbackStorage can be safely shared across threads
// The callback is registered once and the handler is called from ISR context
unsafe impl Sync for Tmp108CallbackStorage {}

/// Per-instance data for a TMP108 sensor.
///
/// This structure is allocated on the heap during init() and a pointer
/// to it is stored in the C struct's rust_ptr field.
///
/// This approach completely decouples the Rust and C struct layouts.
pub struct Tmp108RsData {
    /// TMP108 sensor instance
    sensor: Option<Tmp108<I2cDevice>>,
    /// Last sampled temperature
    sample: i32,
    /// Device ID
    id: u16,
    /// Bus address
    addr: u16,
    /// GPIO alert pin (if configured)
    alert_gpio: Option<GpioPin>,
    /// Sensor trigger handler (stored for callback from GPIO interrupt)
    /// Uses Cell for interior mutability since handler is called from ISR
    trigger_handler: Cell<raw::sensor_trigger_handler_t>,
    /// Device pointer for trigger callbacks
    trigger_device: Cell<*const raw::device>,
    /// Callback storage for GPIO interrupts (contains device context)
    callback_storage: Option<Box<Tmp108CallbackStorage>>,
}

impl Tmp108RsData {
    /// Initialize the data structure to default values
    pub const fn new() -> Self {
        Self {
            sensor: None,
            sample: 2000, // Default to 20.00°C
            id: 0,
            addr: 0,
            alert_gpio: None,
            trigger_handler: Cell::new(None),
            trigger_device: Cell::new(core::ptr::null()),
            callback_storage: None,
        }
    }
}

impl Default for Tmp108RsData {
    fn default() -> Self {
        Self::new()
    }
}

/// TMP108 driver (stateless - all state is in Tmp108RsData)
///
/// This is just a marker type that implements the SensorDriver trait.
/// The actual per-instance state is stored in Tmp108RsData.
pub struct Tmp108Driver;

impl Default for Tmp108Driver {
    fn default() -> Self {
        Self::new()
    }
}

// Generate FFI exports using the sensor framework macro
// This creates:
// - tmp108_rs_driver_api (sensor_driver_api vtable)
// - tmp108_rs_init (init function)
// - Internal FFI wrappers for all sensor operations
zephyr::sensor_ffi_exports!(
    driver: Tmp108Driver,
    prefix: tmp108_rs
);

impl Tmp108Driver {
    /// Create a new TMP108 driver instance (stateless)
    ///
    /// This must be const to allow static initialization.
    pub const fn new() -> Self {
        Self
    }
}

impl SensorDriver for Tmp108Driver {
    fn init(&mut self, dev: DeviceRef) -> SensorResult<()> {
        // SAFETY: dev is guaranteed valid, config points to Tmp108RsConfig
        let cfg = unsafe { dev.config_as::<Tmp108RsConfig>() };

        // SAFETY: Access the C struct that holds the pointer
        let data_ptr = unsafe { dev.data_as_mut::<Tmp108RsDataPtr>() };

        // Allocate the Rust data structure on the heap
        let data = Box::new(Tmp108RsData::new());

        // Store the raw pointer in the C struct
        data_ptr.rust_ptr = Box::into_raw(data);

        // Get a mutable reference to the allocated data
        let data = unsafe { &mut *data_ptr.rust_ptr };

        // Create I2C device from device tree spec
        let i2c = unsafe { I2cDevice::from_dt_spec(&cfg.bus).ok_or(SensorError::InvalidArgument)? };

        // Check if I2C device is ready
        if !i2c.is_ready() {
            info!("I2C device not ready");
            return Err(SensorError::NotReady);
        }

        // Setup alert GPIO if enabled
        if !cfg.alert_gpio.port.is_null() {
            let mut alert_gpio = unsafe {
                GpioPin::from_dt_spec(&cfg.alert_gpio).ok_or(SensorError::InvalidArgument)?
            };

            if !alert_gpio.is_ready() {
                info!("Alert GPIO not ready");
                return Err(SensorError::NotReady);
            }

            alert_gpio.configure_input();

            // Create callback storage with device pointer
            let callback_storage = Box::new(Tmp108CallbackStorage::new(dev.as_ptr()));

            // Register the callback with Zephyr GPIO subsystem
            unsafe {
                let cb_ptr = callback_storage.callback.get();
                let pin_mask = 1u32 << cfg.alert_gpio.pin;

                raw::gpio_init_callback(
                    cb_ptr,
                    Some(Tmp108CallbackStorage::callback_handler),
                    pin_mask,
                );

                let ret = raw::gpio_add_callback(cfg.alert_gpio.port, cb_ptr);
                if ret < 0 {
                    info!("Failed to add GPIO callback");
                    return Err(SensorError::IoError);
                }
            }

            // Enable the GPIO interrupt
            unsafe {
                let ret = raw::gpio_pin_interrupt_configure_dt(
                    &cfg.alert_gpio,
                    raw::ZR_GPIO_INT_EDGE_TO_ACTIVE,
                );
                if ret < 0 {
                    info!("Failed to enable GPIO interrupt: {}", ret);
                    return Err(SensorError::IoError);
                }
            }

            // Store both the GPIO pin and callback storage in per-device data
            data.alert_gpio = Some(alert_gpio);
            data.callback_storage = Some(callback_storage);

            info!("Alert GPIO configured with interrupt handler");
        }

        data.addr = i2c.address();
        info!("TMP108 I2C device created at address 0x{:02X}", data.addr);

        // Validate address and create sensor
        match data.addr {
            0x48 => {
                info!("TMP108 address valid: 0x{:02X}", data.addr);
                // Create and store TMP108 sensor with I2C device
                data.sensor = Some(Tmp108::new_with_a0_gnd(i2c));
            }
            _ => {
                info!("TMP108 address invalid: 0x{:02X}", data.addr);
                return Err(SensorError::InvalidArgument);
            }
        }

        // For now, just set a random device ID
        data.id = 0x1234;

        Ok(())
    }

    fn sample_fetch(&mut self, dev: DeviceRef, channel: SensorChannel) -> SensorResult<()> {
        // SAFETY: Access the C struct that holds the pointer
        let data_ptr = unsafe { dev.data_as::<Tmp108RsDataPtr>() };

        // SAFETY: Dereference the pointer to get the Rust data
        let data = unsafe { &mut *data_ptr.rust_ptr };

        match channel {
            SensorChannel::All | SensorChannel::AmbientTemp => {
                // Get mutable sensor reference
                let sensor = data.sensor.as_mut().ok_or(SensorError::NotReady)?;

                // Read temperature from sensor
                let temp_c = sensor.temperature().map_err(|_| SensorError::IoError)?;

                // Cache the temperature value (convert to units of 0.01°C)
                data.sample = (temp_c * 100.0) as i32;

                info!("Temperature sampled: {} °C", temp_c);
                Ok(())
            }
            _ => Err(SensorError::InvalidChannel),
        }
    }

    fn channel_get(&self, dev: DeviceRef, channel: SensorChannel) -> SensorResult<SensorValue> {
        // SAFETY: Access the C struct that holds the pointer
        let data_ptr = unsafe { dev.data_as::<Tmp108RsDataPtr>() };

        // SAFETY: Dereference the pointer to get the Rust data
        let data = unsafe { &*data_ptr.rust_ptr };

        match channel {
            SensorChannel::AmbientTemp | SensorChannel::All => {
                // Return cached temperature value
                // sample is in units of 0.01°C, convert to millicelsius
                let temp_millicelsius = data.sample * 10;
                Ok(SensorValue::from_millicelsius(temp_millicelsius))
            }
            _ => Err(SensorError::InvalidChannel),
        }
    }

    fn trigger_set(
        &mut self,
        dev: DeviceRef,
        handler: crate::raw::sensor_trigger_handler_t,
    ) -> SensorResult<()> {
        // SAFETY: Access the C struct that holds the pointer
        let data_ptr = unsafe { dev.data_as::<Tmp108RsDataPtr>() };

        // SAFETY: Dereference the pointer to get the Rust data
        let data = unsafe { &mut *data_ptr.rust_ptr };

        // Check if GPIO is configured for triggers
        if data.alert_gpio.is_none() {
            info!("No GPIO pin configured for triggers");
            return Err(SensorError::NotSupported);
        }

        // Store the trigger handler and device pointer
        // These will be accessed by gpio_interrupt_handler when interrupt fires
        data.trigger_handler.set(handler);
        data.trigger_device.set(dev.as_ptr());

        info!("Trigger handler registered");
        Ok(())
    }

    fn attr_set(
        &mut self,
        dev: DeviceRef,
        channel: SensorChannel,
        attr: i32,
        value: &SensorValue,
    ) -> SensorResult<()> {
        // Only support ambient temperature channel
        match channel {
            SensorChannel::AmbientTemp | SensorChannel::All => {}
            _ => return Err(SensorError::InvalidChannel),
        }

        // SAFETY: Access the C struct that holds the pointer
        let data_ptr = unsafe { dev.data_as::<Tmp108RsDataPtr>() };

        // SAFETY: Dereference the pointer to get the Rust data
        let data = unsafe { &mut *data_ptr.rust_ptr };

        // Get mutable sensor reference
        let sensor = data.sensor.as_mut().ok_or(SensorError::NotReady)?;

        match attr as u32 {
            raw::sensor_attribute_SENSOR_ATTR_SAMPLING_FREQUENCY => {
                // TMP108 conversion rate is configured via the Config struct
                // This would require reading config, modifying, and writing back
                // The tmp108 crate uses configure() method with a Config struct
                info!(
                    "Setting sampling frequency to {} Hz - not directly supported",
                    value.val1
                );
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_UPPER_THRESH => {
                // Set high temperature limit for alert
                // val1 = integer degrees, val2 = fractional (millionths)
                let temp_c = value.val1 as f32 + (value.val2 as f32 / 1_000_000.0);
                info!("Setting upper threshold to {} °C", temp_c);

                sensor
                    .set_high_limit(temp_c)
                    .map_err(|_| SensorError::IoError)?;
                Ok(())
            }
            raw::sensor_attribute_SENSOR_ATTR_LOWER_THRESH => {
                // Set low temperature limit for alert
                let temp_c = value.val1 as f32 + (value.val2 as f32 / 1_000_000.0);
                info!("Setting lower threshold to {} °C", temp_c);

                sensor
                    .set_low_limit(temp_c)
                    .map_err(|_| SensorError::IoError)?;
                Ok(())
            }
            raw::sensor_attribute_SENSOR_ATTR_HYSTERESIS => {
                // TMP108 hysteresis is configured via the Config struct, not directly
                info!(
                    "Setting hysteresis to {} °C - not directly supported",
                    value.val1 as f32 + (value.val2 as f32 / 1_000_000.0)
                );
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_OFFSET => {
                // TMP108 doesn't support temperature offset calibration
                info!(
                    "Setting offset to {} °C - not supported by TMP108",
                    value.val1 as f32 + (value.val2 as f32 / 1_000_000.0)
                );
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_ALERT => {
                // Read current configuration
                let mut config = sensor
                    .read_configuration()
                    .map_err(|_| SensorError::IoError)?;

                // Set thermostat mode based on value
                // 0 = Comparator mode (default), non-zero = Interrupt mode
                let enable = value.val1 != 0;
                config.thermostat_mode = if enable {
                    Thermostat::Interrupt
                } else {
                    Thermostat::Comparator
                };

                info!(
                    "Setting alert mode: {}",
                    if enable { "interrupt" } else { "comparator" }
                );

                // Write updated configuration back
                sensor.configure(config).map_err(|_| SensorError::IoError)?;
                Ok(())
            }
            _ => {
                info!("Unsupported attribute: {}", attr);
                Err(SensorError::NotSupported)
            }
        }
    }

    fn attr_get(
        &self,
        dev: DeviceRef,
        channel: SensorChannel,
        attr: i32,
    ) -> SensorResult<SensorValue> {
        // Only support ambient temperature channel
        match channel {
            SensorChannel::AmbientTemp | SensorChannel::All => {}
            _ => return Err(SensorError::InvalidChannel),
        }

        // SAFETY: Access the C struct that holds the pointer
        let data_ptr = unsafe { dev.data_as::<Tmp108RsDataPtr>() };

        // SAFETY: Dereference the pointer to get the Rust data
        let data = unsafe { &*data_ptr.rust_ptr };

        // Verify sensor is initialized
        let _sensor = data.sensor.as_ref().ok_or(SensorError::NotReady)?;

        // Note: high_limit() and low_limit() require mutable reference in tmp108 crate
        // We need to work around this by using interior mutability or accepting the limitation
        // For now, we'll return NotSupported for get operations that require mutation

        match attr as u32 {
            raw::sensor_attribute_SENSOR_ATTR_SAMPLING_FREQUENCY => {
                // TMP108 conversion rate is in the Config register
                // Would need read_configuration() which requires &mut self
                info!("Getting sampling frequency - not directly supported");
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_UPPER_THRESH => {
                // high_limit() requires &mut self in the tmp108 crate
                // This is a limitation - would need interior mutability to support
                info!("Getting upper threshold - requires mutable access");
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_LOWER_THRESH => {
                // low_limit() requires &mut self in the tmp108 crate
                info!("Getting lower threshold - requires mutable access");
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_HYSTERESIS => {
                // Hysteresis is part of Config register
                info!("Getting hysteresis - not directly supported");
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_OFFSET => {
                // TMP108 doesn't support offset
                info!("Getting offset - not supported by TMP108");
                Err(SensorError::NotSupported)
            }
            raw::sensor_attribute_SENSOR_ATTR_ALERT => {
                // Alert status is in Config register
                info!("Getting alert mode - not directly supported");
                Err(SensorError::NotSupported)
            }
            _ => {
                info!("Unsupported attribute: {}", attr);
                Err(SensorError::NotSupported)
            }
        }
    }
}
