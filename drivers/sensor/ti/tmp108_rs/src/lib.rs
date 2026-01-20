#![no_std]
#![no_main]

//! TMP108 Temperature Sensor Driver
//!
//! This driver provides support for Texas Instruments TMP108 series
//! temperature sensors using Zephyr's generic sensor framework in Rust.

use log::info;
use zephyr::sensor::{SensorError, SensorResult};

/// FFI binding for tmp108_rs_config struct from C code.
///
/// This must match the C struct definition exactly.
#[repr(C)]
pub struct Tmp108RsConfig {
    /// I2C device tree spec
    pub bus: zephyr::raw::i2c_dt_spec,
}

/// TMP108 driver state
///
/// This structure holds the runtime state for a TMP108 sensor instance.
/// All sensor logic is implemented in safe Rust.
///
pub struct Tmp108Driver {
    /// Last sampled temperature
    sample: i32,
    /// Device ID
    id: u16,
}

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
    /// Create a new TMP108 driver instance
    ///
    /// This must be const to allow static initialization.
    pub const fn new() -> Self {
        Self {
            sample: 2000, // Default to 20.00°C
            id: 0,
        }
    }
}

impl SensorDriver for Tmp108Driver {
    fn init(&mut self, dev: DeviceRef, device_ready: bool) -> SensorResult<()> {
        if !device_ready {
            return Err(SensorError::NotReady);
        }

        // SAFETY: dev is guaranteed valid, config points to Tmp108RsConfig
        let cfg = unsafe { dev.config_as::<Tmp108RsConfig>() };

        // Now you can access the i2c_dt_spec bus information
        let _i2c_bus = &cfg.bus;

        // For now, just set a random device ID
        self.id = 0x1234;

        Ok(())
    }

    fn sample_fetch(&mut self, _dev: DeviceRef, channel: SensorChannel) -> SensorResult<()> {
        match channel {
            SensorChannel::All | SensorChannel::AmbientTemp => {
                info!("AmbientTemp read requested");
                Ok(())
            }
            _ => Err(SensorError::InvalidChannel),
        }
    }

    fn channel_get(&self, channel: SensorChannel) -> SensorResult<SensorValue> {
        match channel {
            SensorChannel::AmbientTemp | SensorChannel::All => {
                // Convert stored sample to SensorValue
                // sample is in units of 0.01°C, convert to millicelsius
                let temp_c = self.sample * 10;
                Ok(SensorValue::from_millicelsius(temp_c))
            }
            _ => Err(SensorError::InvalidChannel),
        }
    }

    // attr_set and attr_get use default implementations (NotSupported)
    // Override these if you need to support sensor attributes
}

unsafe extern "C" {
    pub fn tmp11x_reg_read_wrapper(
        ptr: *const core::ffi::c_void,
        reg: u8,
        val: *mut u16,
    ) -> core::ffi::c_int;
}

/// Read a register from TMP108 via I2C using C implementation.
///
/// This is a thin wrapper while a Rust-based I2C implementation is developed.
/// # Safety
/// - `ptr` must be a valid pointer to the Zephyr I2C device context expected by the C side.
pub unsafe fn tmp11x_reg_read(ptr: *const core::ffi::c_void, reg: u8) -> SensorResult<u16> {
    let mut raw: u16 = 1;

    if ptr.is_null() {
        return Err(SensorError::InvalidArgument); // -EINVAL
    }

    // SAFETY: ptr is checked for null above
    let rc = unsafe { tmp11x_reg_read_wrapper(ptr, reg, &mut raw) };
    if rc < 0 {
        return Err(SensorError::IoError); // -EIO
    }

    Ok(raw)
}
