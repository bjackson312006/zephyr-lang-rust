#![no_std]
#![no_main]

//! TMP108 Temperature Sensor Driver
//!
//! This driver provides support for Texas Instruments TMP108 series
//! temperature sensors using Zephyr's generic sensor framework in Rust.

use log::info;
use tmp108::Tmp108;
use zephyr::i2c::I2cDevice;
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
    /// TMP108 sensor instance
    sensor: Option<Tmp108<I2cDevice>>,
    /// Last sampled temperature
    sample: i32,
    /// Device ID
    id: u16,
    /// Bus address
    addr: u16,
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
            sensor: None,
            sample: 2000, // Default to 20.00°C
            id: 0,
            addr: 0,
        }
    }
}

impl SensorDriver for Tmp108Driver {
    fn init(&mut self, dev: DeviceRef) -> SensorResult<()> {
        // SAFETY: dev is guaranteed valid, config points to Tmp108RsConfig
        let cfg = unsafe { dev.config_as::<Tmp108RsConfig>() };

        // Create I2C device from device tree spec
        let i2c = unsafe {
            I2cDevice::from_dt_spec(&cfg.bus)
                .ok_or(SensorError::InvalidArgument)?
        };

        // Check if I2C device is ready
        if !i2c.is_ready() {
            info!("I2C device not ready");
            return Err(SensorError::NotReady);
        }

        self.addr = i2c.address();
        info!("TMP108 I2C device created at address 0x{:02X}", self.addr);

        // Validate address and create sensor
        match self.addr {
            0x48 => {
                info!("TMP108 address valid: 0x{:02X}", self.addr);
                // Create and store TMP108 sensor with I2C device
                self.sensor = Some(Tmp108::new_with_a0_gnd(i2c));
            }
            _ => {
                info!("TMP108 address invalid: 0x{:02X}", self.addr);
                return Err(SensorError::InvalidArgument);
            }
        }

        // For now, just set a random device ID
        self.id = 0x1234;

        Ok(())
    }

    fn sample_fetch(&mut self, _dev: DeviceRef, channel: SensorChannel) -> SensorResult<()> {
        match channel {
            SensorChannel::All | SensorChannel::AmbientTemp => {
                // Get mutable sensor reference
                let sensor = self.sensor.as_mut().ok_or(SensorError::NotReady)?;

                // Read temperature from sensor
                let temp_c = sensor.temperature().map_err(|_| SensorError::IoError)?;

                // Cache the temperature value (convert to units of 0.01°C)
                self.sample = (temp_c * 100.0) as i32;

                info!("Temperature sampled: {} °C", temp_c);
                Ok(())
            }
            _ => Err(SensorError::InvalidChannel),
        }
    }

    fn channel_get(&self, channel: SensorChannel) -> SensorResult<SensorValue> {
        match channel {
            SensorChannel::AmbientTemp | SensorChannel::All => {
                // Return cached temperature value
                // sample is in units of 0.01°C, convert to millicelsius
                let temp_millicelsius = self.sample * 10;
                Ok(SensorValue::from_millicelsius(temp_millicelsius))
            }
            _ => Err(SensorError::InvalidChannel),
        }
    }

    // attr_set and attr_get use default implementations (NotSupported)
    // Override these if you need to support sensor attributes
}
