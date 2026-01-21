#![no_std]
#![no_main]

//! TMP11x Temperature Sensor Driver
//!
//! This driver provides support for Texas Instruments TMP11x series
//! temperature sensors using Zephyr's generic sensor framework in Rust.

use log::info;

use embedded_sensors_hal::sensor;
use embedded_sensors_hal::temperature::{DegreesCelsius, TemperatureSensor};
use zephyr::i2c::I2cDevice;
use zephyr::sensor::{SensorError, SensorResult};

/// FFI binding for tmp11x_dev_config struct from C code.
///
/// This must match the C struct definition exactly.
#[repr(C)]
pub struct Tmp11xDevConfig {
    /// I2C device tree spec
    pub bus: zephyr::raw::i2c_dt_spec,
    /// Output data rate
    pub odr: u16,
    /// Oversampling configuration
    pub oversampling: u16,
    /// Alert pin polarity
    pub alert_pin_polarity: bool,
    /// Alert mode
    pub alert_mode: bool,
    /// Alert data ready select
    pub alert_dr_sel: bool,
    /// Store attribute values
    pub store_attr_values: bool,
    // Note: alert_gpio (gpio_dt_spec) is conditionally compiled with CONFIG_TMP11X_RS_TRIGGER
    // If needed, it can be added with #[cfg(feature = "trigger")]
}

/// TMP11x driver state
///
/// This structure holds the runtime state for a TMP11x sensor instance.
/// All sensor logic is implemented in safe Rust.
///
pub struct Tmp11xDriver {
    /// Temperature sensor instance
    sensor: Option<TempSensorTmp11x>,
    /// Last sampled temperature
    sample: i32,
    /// Device ID
    id: u16,
    /// Bus address
    addr: u16,
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
            sensor: None,
            sample: 2000, // Default to 20.00°C
            id: 0,
            addr: 0,
        }
    }
}

impl SensorDriver for Tmp11xDriver {
    fn init(&mut self, dev: DeviceRef) -> SensorResult<()> {
        // SAFETY: dev is guaranteed valid, config points to Tmp11xDevConfig
        let cfg = unsafe { dev.config_as::<Tmp11xDevConfig>() };

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

        // Store the I2C bus address
        self.addr = i2c.address();

        info!("TMP11x detected at I2C address 0x{:02X}", self.addr);

        // Create and store the temperature sensor
        self.sensor = Some(TempSensorTmp11x::new(i2c));

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
                let temperature = sensor.temperature()
                    .map_err(|e| {
                        info!("Error reading temperature {:?}", e);
                        SensorError::IoError
                    })?;

                // Cache the temperature value (convert to units of 0.01°C)
                self.sample = temperature as i32;

                info!("Temperature sampled: {} °C", temperature);
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

/// Embedded HAL Temperature Sensor for TMP11x
///
/// This struct provides a safe Rust embedded TemperatureSensor interface
/// to the TMP11x temperature sensor.
pub struct TempSensorTmp11x {
    i2c: I2cDevice,
}

impl TempSensorTmp11x {
    /// Create a new TMP11x temperature sensor
    pub fn new(i2c: I2cDevice) -> Self {
        Self { i2c }
    }

    /// Read a register from TMP11x via I2C
    fn read_register(&mut self, reg: u8) -> Result<u16, TempSensorTmp11xError> {
        let mut buf = [0u8; 2];

        // Write register address, then read 2 bytes
        self.i2c
            .write_read(&[reg], &mut buf)
            .map_err(|_| TempSensorTmp11xError::Bus)?;

        // TMP11x returns data in big-endian format
        Ok(u16::from_be_bytes(buf))
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
        let temp_raw = self.read_register(0x00)?;

        // Convert raw value to degrees Celsius
        // TMP11x stores temperature in the upper 12 bits (or 13 for extended mode)
        // For 12-bit resolution: temp = raw_value / 16 * 0.0625°C
        Ok(temp_raw.into())
    }
}
