//! Generic sensor types - reusable for all sensor drivers
//!
//! This module provides generic types that mirror Zephyr RTOS sensor subsystem structures,
//! enabling Rust code to interact with sensor hardware through safe abstractions.
//! These types are designed to be reusable across different sensor driver implementations.

/// Sensor channel types (matches Zephyr's `sensor_channel` enum)
///
/// Represents different types of sensor channels that can be sampled or configured.
/// Each channel corresponds to a specific measurement type (e.g., temperature, pressure,
/// color components). The numeric values match Zephyr's C enum definitions to ensure
/// correct FFI interoperability.
///
/// # Examples
///
/// ```no_run
/// use zephyr::sensor::types::SensorChannel;
///
/// // Request all channels
/// let channel = SensorChannel::All;
///
/// // Request specific temperature channel
/// let temp_channel = SensorChannel::AmbientTemp;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum SensorChannel {
    /// All available sensor channels
    ///
    /// Used to fetch or configure all channels at once. This is commonly used
    /// when you want to sample all available measurements from a sensor.
    All = 0,

    /// Red color channel
    ///
    /// Used by color/light sensors to measure red light intensity.
    Red = 1,

    /// Green color channel
    ///
    /// Used by color/light sensors to measure green light intensity.
    Green = 2,

    /// Blue color channel
    ///
    /// Used by color/light sensors to measure blue light intensity.
    Blue = 3,

    /// Overall light intensity
    ///
    /// Measures total light intensity across the visible spectrum.
    /// Used by ambient light sensors.
    Intensity = 4,

    /// Infrared channel
    ///
    /// Measures infrared light intensity. Used by proximity sensors
    /// and IR light sensors.
    Ir = 5,

    /// Ambient temperature
    ///
    /// Measures the temperature of the surrounding environment.
    /// Common in environmental sensors like BME280 or TMP11x.
    AmbientTemp = 13,

    /// Die (chip) temperature
    ///
    /// Measures the temperature of the sensor chip itself.
    /// Useful for temperature compensation or thermal monitoring.
    DieTemp = 14,

    /// Accelerometer data (3-axis)
    ///
    /// Measures linear acceleration in X, Y, and Z axes.
    /// Used by accelerometer sensors like ADXL345 or MPU6050.
    Accel = 15,

    /// Gyroscope data (3-axis)
    ///
    /// Measures angular velocity around X, Y, and Z axes.
    /// Used by gyroscope sensors like MPU6050 or L3GD20.
    Gyro = 16,

    /// Magnetometer data (3-axis)
    ///
    /// Measures magnetic field strength in X, Y, and Z axes.
    /// Used by magnetometer sensors for compass applications.
    Magn = 17,

    /// Relative humidity
    ///
    /// Measures the percentage of water vapor in the air.
    /// Common in environmental sensors like BME280 or SHT3x.
    Humidity = 22,

    /// Atmospheric pressure
    ///
    /// Measures barometric pressure. Used by pressure sensors
    /// like BMP280 or BME280 for weather monitoring or altitude estimation.
    Pressure = 23,

    /// Proximity detection
    ///
    /// Measures the proximity of nearby objects. Used by proximity
    /// sensors for presence detection or touchless interfaces.
    Prox = 24,

    /// Distance measurement
    ///
    /// Measures absolute distance to objects. Used by time-of-flight
    /// or ultrasonic distance sensors.
    Distance = 25,
}

/// Converts a raw integer channel ID to a `SensorChannel` enum variant
///
/// This implementation enables conversion from Zephyr's C integer channel values
/// to the type-safe Rust enum. Unknown or invalid channel IDs default to `All`.
///
/// # Examples
///
/// ```
/// use zephyr::sensor::types::SensorChannel;
///
/// let channel = SensorChannel::from(13);
/// assert_eq!(channel, SensorChannel::AmbientTemp);
///
/// // Invalid channels default to All
/// let unknown = SensorChannel::from(999);
/// assert_eq!(unknown, SensorChannel::All);
/// ```
impl From<i32> for SensorChannel {
    fn from(chan: i32) -> Self {
        match chan {
            0 => SensorChannel::All,
            1 => SensorChannel::Red,
            2 => SensorChannel::Green,
            3 => SensorChannel::Blue,
            4 => SensorChannel::Intensity,
            5 => SensorChannel::Ir,
            13 => SensorChannel::AmbientTemp,
            14 => SensorChannel::DieTemp,
            15 => SensorChannel::Accel,
            16 => SensorChannel::Gyro,
            17 => SensorChannel::Magn,
            22 => SensorChannel::Humidity,
            23 => SensorChannel::Pressure,
            24 => SensorChannel::Prox,
            25 => SensorChannel::Distance,
            _ => SensorChannel::All, // Default fallback
        }
    }
}

/// Sensor value (mirrors Zephyr's `struct sensor_value`)
///
/// Represents a sensor reading as two 32-bit integers: an integer part (`val1`)
/// and a fractional part (`val2`). This structure matches Zephyr's sensor value
/// representation for FFI compatibility.
///
/// The interpretation of `val1` and `val2` depends on the sensor type:
/// - Temperature: `val1` = degrees, `val2` = micro-degrees
/// - Pressure: `val1` = pascals, `val2` = micro-pascals
/// - Humidity: `val1` = percentage, `val2` = fractional percentage
///
/// # Examples
///
/// ```
/// use zephyr::sensor::types::SensorValue;
///
/// // Represent 25.5°C
/// let temp = SensorValue::new(25, 500_000);
///
/// // Or use convenience methods
/// let temp = SensorValue::from_millicelsius(25_500);
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct SensorValue {
    /// Integer part of the value
    pub val1: i32,
    /// Fractional part of the value (typically in micro-units)
    pub val2: i32,
}

impl SensorValue {
    /// Creates a new `SensorValue` with the given integer and fractional parts
    ///
    /// # Arguments
    ///
    /// * `val1` - The integer part of the value
    /// * `val2` - The fractional part of the value
    ///
    /// # Examples
    ///
    /// ```
    /// use zephyr::sensor::types::SensorValue;
    ///
    /// let value = SensorValue::new(42, 123_456);
    /// ```
    pub const fn new(val1: i32, val2: i32) -> Self {
        Self { val1, val2 }
    }

    /// Creates a `SensorValue` from a temperature in millicelsius
    ///
    /// Converts a temperature value in millicelsius (1/1000th of a degree)
    /// to the two-part representation. For example, 25_500 millicelsius
    /// represents 25.5°C.
    ///
    /// # Arguments
    ///
    /// * `mc` - Temperature in millicelsius
    ///
    /// # Examples
    ///
    /// ```
    /// use zephyr::sensor::types::SensorValue;
    ///
    /// // 25.5°C = 25,500 millicelsius
    /// let temp = SensorValue::from_millicelsius(25_500);
    /// assert_eq!(temp.val1, 25);
    /// assert_eq!(temp.val2, 500_000);
    /// ```
    pub fn from_millicelsius(mc: i32) -> Self {
        Self {
            val1: mc / 1000,
            val2: (mc % 1000) * 1000,
        }
    }
}

/// Driver error type
///
/// Represents errors that can occur during sensor operations. Each variant
/// corresponds to a specific failure condition and maps to a standard
/// Zephyr errno value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorError {
    /// Sensor is not ready or not responding
    ///
    /// Indicates the sensor device is not available, possibly due to:
    /// - Device not being initialized
    /// - Hardware failure or disconnection
    /// - Device in sleep or low-power mode
    ///
    /// Maps to errno `-ENODEV` (-19).
    NotReady,

    /// Invalid or unsupported channel requested
    ///
    /// The requested sensor channel is not valid for this sensor or
    /// not supported by the driver.
    ///
    /// Maps to errno `-EINVAL` (-22).
    InvalidChannel,

    /// I/O error during communication
    ///
    /// A hardware I/O error occurred during sensor communication,
    /// such as I²C or SPI bus failures.
    ///
    /// Maps to errno `-EIO` (-5).
    IoError,

    /// Invalid argument provided to function
    ///
    /// An invalid parameter was passed to a sensor function.
    ///
    /// Maps to errno `-EINVAL` (-22).
    InvalidArgument,

    /// Operation not supported by this sensor
    ///
    /// The requested operation is not implemented or supported
    /// by this particular sensor driver.
    ///
    /// Maps to errno `-ENOTSUP` (-95).
    NotSupported,
}

impl SensorError {
    /// Converts the error to its corresponding Zephyr errno value
    ///
    /// Returns the negative errno value that corresponds to this error type,
    /// matching Zephyr's error code conventions.
    ///
    /// # Examples
    ///
    /// ```
    /// use zephyr::sensor::types::SensorError;
    ///
    /// let err = SensorError::IoError;
    /// assert_eq!(err.to_errno(), -5); // -EIO
    /// ```
    pub const fn to_errno(self) -> i32 {
        match self {
            SensorError::NotReady => -19,       // -ENODEV
            SensorError::InvalidChannel => -22, // -EINVAL
            SensorError::IoError => -5,         // -EIO
            SensorError::InvalidArgument => -22,
            SensorError::NotSupported => -95, // -ENOTSUP
        }
    }
}

/// Result type for sensor operations
///
/// A type alias for `Result<T, SensorError>`, providing a convenient way to
/// return results from sensor driver functions that may fail.
///
/// # Examples
///
/// ```
/// use zephyr::sensor::types::{SensorResult, SensorError, SensorValue};
///
/// fn read_temperature() -> SensorResult<SensorValue> {
///     // ... sensor reading logic ...
///     Ok(SensorValue::new(25, 500_000))
/// }
/// ```
pub type SensorResult<T> = Result<T, SensorError>;

// Sensor utility functions - FFI bindings to C helper functions
// These match the static inline functions in zephyr/include/zephyr/drivers/sensor.h

/// The value of gravitational constant in micro m/s^2.
pub const SENSOR_G: i64 = 9_806_650;

/// The value of constant PI in micros.
pub const SENSOR_PI: i64 = 3_141_592;

impl SensorValue {
    /// Helper function to convert acceleration from m/s^2 to Gs
    ///
    /// # Arguments
    ///
    /// * `ms2` - A reference to a sensor_value struct holding the acceleration, in m/s^2.
    ///
    /// # Returns
    ///
    /// The converted value, in Gs.
    pub fn ms2_to_g(&self) -> i32 {
        let micro_ms2 = self.val1 as i64 * 1_000_000 + self.val2 as i64;

        if micro_ms2 > 0 {
            ((micro_ms2 + SENSOR_G / 2) / SENSOR_G) as i32
        } else {
            ((micro_ms2 - SENSOR_G / 2) / SENSOR_G) as i32
        }
    }

    /// Helper function to convert acceleration from Gs to m/s^2
    ///
    /// # Arguments
    ///
    /// * `g` - The G value to be converted.
    ///
    /// # Returns
    ///
    /// A SensorValue holding the result in m/s^2.
    pub fn g_to_ms2(g: i32) -> Self {
        let result = g as i64 * SENSOR_G;
        Self {
            val1: (result / 1_000_000) as i32,
            val2: (result % 1_000_000) as i32,
        }
    }

    /// Helper function to convert acceleration from m/s^2 to milli Gs
    ///
    /// # Arguments
    ///
    /// * `ms2` - A reference to a sensor_value struct holding the acceleration, in m/s^2.
    ///
    /// # Returns
    ///
    /// The converted value, in milli Gs.
    pub fn ms2_to_mg(&self) -> i32 {
        let nano_ms2 = (self.val1 as i64 * 1_000_000 + self.val2 as i64) * 1_000;

        if nano_ms2 > 0 {
            ((nano_ms2 + SENSOR_G / 2) / SENSOR_G) as i32
        } else {
            ((nano_ms2 - SENSOR_G / 2) / SENSOR_G) as i32
        }
    }

    /// Helper function to convert acceleration from m/s^2 to micro Gs
    ///
    /// # Arguments
    ///
    /// * `ms2` - A reference to a sensor_value struct holding the acceleration, in m/s^2.
    ///
    /// # Returns
    ///
    /// The converted value, in micro Gs.
    pub fn ms2_to_ug(&self) -> i32 {
        let micro_ms2 = self.val1 as i64 * 1_000_000 + self.val2 as i64;

        ((micro_ms2 * 1_000_000) / SENSOR_G) as i32
    }

    /// Helper function to convert acceleration from micro Gs to m/s^2
    ///
    /// # Arguments
    ///
    /// * `ug` - The micro G value to be converted.
    ///
    /// # Returns
    ///
    /// A SensorValue holding the result in m/s^2.
    pub fn ug_to_ms2(ug: i32) -> Self {
        let result = ug as i64 * SENSOR_G / 1_000_000;
        Self {
            val1: (result / 1_000_000) as i32,
            val2: (result % 1_000_000) as i32,
        }
    }

    /// Helper function for converting radians to degrees.
    ///
    /// # Arguments
    ///
    /// * `rad` - A reference to a sensor_value struct, holding the value in radians.
    ///
    /// # Returns
    ///
    /// The converted value, in degrees.
    pub fn rad_to_degrees(&self) -> i32 {
        let micro_rad_s = self.val1 as i64 * 1_000_000 + self.val2 as i64;

        if micro_rad_s > 0 {
            ((micro_rad_s * 180 + SENSOR_PI / 2) / SENSOR_PI) as i32
        } else {
            ((micro_rad_s * 180 - SENSOR_PI / 2) / SENSOR_PI) as i32
        }
    }

    /// Helper function for converting degrees to radians.
    ///
    /// # Arguments
    ///
    /// * `d` - The value (in degrees) to be converted.
    ///
    /// # Returns
    ///
    /// A SensorValue holding the result in radians.
    pub fn degrees_to_rad(d: i32) -> Self {
        let result = d as i64 * SENSOR_PI / 180;
        Self {
            val1: (result / 1_000_000) as i32,
            val2: (result % 1_000_000) as i32,
        }
    }

    /// Helper function for converting radians to 10 micro degrees.
    ///
    /// When the unit is 1 micro degree, the range that the int32_t can represent is
    /// +/-2147.483 degrees. In order to increase this range, here we use 10 micro
    /// degrees as the unit.
    ///
    /// # Arguments
    ///
    /// * `rad` - A reference to a sensor_value struct, holding the value in radians.
    ///
    /// # Returns
    ///
    /// The converted value, in 10 micro degrees.
    pub fn rad_to_10udegrees(&self) -> i32 {
        let micro_rad_s = self.val1 as i64 * 1_000_000 + self.val2 as i64;

        ((micro_rad_s * 180 * 100_000) / SENSOR_PI) as i32
    }

    /// Helper function for converting 10 micro degrees to radians.
    ///
    /// # Arguments
    ///
    /// * `d` - The value (in 10 micro degrees) to be converted.
    ///
    /// # Returns
    ///
    /// A SensorValue holding the result in radians.
    pub fn _10udegrees_to_rad(d: i32) -> Self {
        let result = d as i64 * SENSOR_PI / 180 / 100_000;
        Self {
            val1: (result / 1_000_000) as i32,
            val2: (result % 1_000_000) as i32,
        }
    }

    /// Helper function for converting struct sensor_value to double.
    ///
    /// # Returns
    ///
    /// The converted value as f64.
    pub fn to_double(&self) -> f64 {
        self.val1 as f64 + self.val2 as f64 / 1_000_000.0
    }

    /// Helper function for converting struct sensor_value to float.
    ///
    /// # Returns
    ///
    /// The converted value as f32.
    pub fn to_float(&self) -> f32 {
        self.val1 as f32 + self.val2 as f32 / 1_000_000.0
    }

    /// Helper function for converting double to struct sensor_value.
    ///
    /// # Arguments
    ///
    /// * `inp` - The value to convert.
    ///
    /// # Returns
    ///
    /// Ok(SensorValue) if successful, Err if the value is out of range.
    pub fn from_double(inp: f64) -> Result<Self, ()> {
        if inp < i32::MIN as f64 || inp > i32::MAX as f64 {
            return Err(());
        }

        let val1 = inp as i32;
        let val2 = ((inp - val1 as f64) * 1_000_000.0) as i32;

        Ok(Self { val1, val2 })
    }

    /// Helper function for converting float to struct sensor_value.
    ///
    /// # Arguments
    ///
    /// * `inp` - The value to convert.
    ///
    /// # Returns
    ///
    /// Ok(SensorValue) if successful, Err if the value is out of range.
    pub fn from_float(inp: f32) -> Result<Self, ()> {
        if inp < i32::MIN as f32 || inp >= i32::MAX as f32 {
            return Err(());
        }

        let val1 = inp as i32;
        let val2 = ((inp - val1 as f32) * 1_000_000.0) as i32;

        Ok(Self { val1, val2 })
    }

    /// Helper function for converting struct sensor_value to integer deci units.
    ///
    /// # Returns
    ///
    /// The converted value in deci units (1/10).
    pub fn to_deci(&self) -> i64 {
        self.val1 as i64 * 10 + self.val2 as i64 / 100_000
    }

    /// Helper function for converting struct sensor_value to integer centi units.
    ///
    /// # Returns
    ///
    /// The converted value in centi units (1/100).
    pub fn to_centi(&self) -> i64 {
        self.val1 as i64 * 100 + self.val2 as i64 / 10_000
    }

    /// Helper function for converting struct sensor_value to integer milli units.
    ///
    /// # Returns
    ///
    /// The converted value in milli units (1/1000).
    pub fn to_milli(&self) -> i64 {
        self.val1 as i64 * 1_000 + self.val2 as i64 / 1_000
    }

    /// Helper function for converting struct sensor_value to integer micro units.
    ///
    /// # Returns
    ///
    /// The converted value in micro units (1/1000000).
    pub fn to_micro(&self) -> i64 {
        self.val1 as i64 * 1_000_000 + self.val2 as i64
    }

    /// Helper function for converting integer milli units to struct sensor_value.
    ///
    /// # Arguments
    ///
    /// * `milli` - The value in milli units to convert.
    ///
    /// # Returns
    ///
    /// Ok(SensorValue) if successful, Err if the value is out of range.
    pub fn from_milli(milli: i64) -> Result<Self, ()> {
        if milli < (i32::MIN as i64 - 1) * 1_000 || milli > (i32::MAX as i64 + 1) * 1_000 {
            return Err(());
        }

        Ok(Self {
            val1: (milli / 1_000) as i32,
            val2: (milli % 1_000) as i32 * 1_000,
        })
    }

    /// Helper function for converting integer micro units to struct sensor_value.
    ///
    /// # Arguments
    ///
    /// * `micro` - The value in micro units to convert.
    ///
    /// # Returns
    ///
    /// Ok(SensorValue) if successful, Err if the value is out of range.
    pub fn from_micro(micro: i64) -> Result<Self, ()> {
        if micro < (i32::MIN as i64 - 1) * 1_000_000 || micro > (i32::MAX as i64 + 1) * 1_000_000 {
            return Err(());
        }

        Ok(Self {
            val1: (micro / 1_000_000) as i32,
            val2: (micro % 1_000_000) as i32,
        })
    }
}
