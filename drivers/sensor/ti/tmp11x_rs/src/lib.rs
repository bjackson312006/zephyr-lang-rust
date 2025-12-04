#![no_std]
#![no_main]

//! TMP11x Temperature Sensor Driver
//!
//! This driver provides support for Texas Instruments TMP11x series
//! temperature sensors using Zephyr's generic sensor framework in Rust.

use core::ffi::c_void;
use log::info;

use embedded_sensors_hal::sensor;
use embedded_sensors_hal::temperature::{DegreesCelsius, TemperatureSensor};
use zephyr::sensor::{SensorError, SensorResult};

/// TMP11x driver state
///
/// This structure holds the runtime state for a TMP11x sensor instance.
/// All sensor logic is implemented in safe Rust.
///
pub struct Tmp11xDriver {
    /// Last sampled temperature
    sample: i32,
    /// Device ID
    id: u16,
}

impl Default for Tmp11xDriver {
    fn default() -> Self {
        Self::new()
    }
}

// Generate FFI exports using the sensor framework macro
// This creates:
// - tmp11x_rs_driver_api (sensor_driver_api vtable)
// - tmp11x_rs_init (init function)
// - Internal FFI wrappers for all sensor operations
zephyr::sensor_ffi_exports!(
    driver: Tmp11xDriver,
    prefix: tmp11x_rs
);

impl Tmp11xDriver {
    /// Create a new TMP11x driver instance
    ///
    /// This must be const to allow static initialization.
    pub const fn new() -> Self {
        Self {
            sample: 2000, // Default to 20.00°C
            id: 0,
        }
    }
}

impl SensorDriver for Tmp11xDriver {
    fn init(&mut self, _dev: *const zephyr::raw::device, device_ready: bool) -> SensorResult<()> {
        if !device_ready {
            return Err(SensorError::NotReady);
        }

        // In a real implementation, we would:
        // 1. Read and verify the device ID from register
        // 2. Configure the sensor (conversion rate, alert settings, etc.)
        // 3. Perform any necessary calibration

        // For now, just set a random device ID
        self.id = 0x1234;

        Ok(())
    }

    fn sample_fetch(&mut self, dev: *const zephyr::raw::device, channel: SensorChannel) -> SensorResult<()> {
        match channel {
            SensorChannel::All | SensorChannel::AmbientTemp => {
                // In a real implementation, we would:
                // 1. Read the temperature register via I2C
                // 2. Convert the raw value to temperature
                // 3. Store in self.sample

                let mut sensor = match TempSensorTmp11x::try_new(dev as *const c_void) {
                    Ok(sensor) => sensor,
                    Err(_) => {
                        info!("Error creating TempSensorTmp11x");
                        return Err(SensorError::InvalidArgument); // -EINVAL
                    }
                };

                self.sample = match sensor.temperature() {
                    Ok(temperature) => temperature as i32,
                    Err(e) => {
                        info!("Error reading temperature {:?}", e);
                        return Err(SensorError::IoError); // -EIO
                    }
                };

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

/// Read a register from TMP11x via I2C using C implementation.
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

/// Embedded HAL Temperature Sensor for TMP11x
///
/// This struct provides a safe Rust embedded TemperatureSensor interface
/// to the TMP11x temperature sensor.
pub struct TempSensorTmp11x {
    ptr: *const core::ffi::c_void,
}

impl TempSensorTmp11x {
    const fn try_new(ptr: *const core::ffi::c_void) -> Result<Self, TempSensorTmp11xError> {
        if ptr.is_null() {
            return Err(TempSensorTmp11xError::Invalid);
        }

        Ok(Self { ptr })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TempSensorTmp11xError {
    Bus,
    Invalid,
    Unknown,
}

impl sensor::Error for TempSensorTmp11xError {
    fn kind(&self) -> sensor::ErrorKind {
        embedded_sensors_hal::sensor::ErrorKind::Other
    }
}

impl sensor::ErrorType for TempSensorTmp11x {
    type Error = TempSensorTmp11xError;
}

impl TemperatureSensor for TempSensorTmp11x {
    fn temperature(&mut self) -> Result<DegreesCelsius, Self::Error> {
        // Read temperature register (0x00) TMP11X_REG_TEMP
        let temp_raw =
            unsafe { tmp11x_reg_read(self.ptr, 0x00).map_err(|_| TempSensorTmp11xError::Bus)? };

        Ok(temp_raw.into())
    }
}
