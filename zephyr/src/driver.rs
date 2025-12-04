//! Zephyr driver support
//!
//! This module provides traits and types for implementing Zephyr device drivers in Rust.

use core::ffi::c_int;
use crate::raw;

/// A reference to a Zephyr device instance
pub struct Device {
    pub(crate) raw: *const raw::device,
}

impl Device {
    /// Create a device from a raw pointer (used by generated code)
    #[doc(hidden)]
    pub const unsafe fn from_raw(raw: *const raw::device) -> Self {
        Self { raw }
    }

    /// Get the raw device pointer
    pub fn as_raw(&self) -> *const raw::device {
        self.raw
    }

    /// Check if the device is ready
    pub fn is_ready(&self) -> bool {
        unsafe { raw::device_is_ready(self.raw) }
    }
}

/// Sensor channel types
///
/// Represents the different types of sensor channels that can be read from a sensor device.
/// These map directly to Zephyr's `sensor_channel` enum values.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorChannel {
    /// Ambient temperature channel
    ///
    /// Used for reading temperature sensors. The value is typically expressed in degrees Celsius,
    /// with `val1` representing the integer part and `val2` representing the fractional part
    /// in millionths (e.g., 25.5°C = val1: 25, val2: 500000).
    ///
    /// Maps to Zephyr's `SENSOR_CHAN_AMBIENT_TEMP`.
    AmbientTemp = raw::sensor_channel_SENSOR_CHAN_AMBIENT_TEMP as i32,

    /// All sensor channels
    ///
    /// Used in `sample_fetch()` to indicate that all available sensor channels should be sampled.
    /// When used in `channel_get()`, it typically refers to the primary or default channel.
    ///
    /// Maps to Zephyr's `SENSOR_CHAN_ALL`.
    All = raw::sensor_channel_SENSOR_CHAN_ALL as i32,
    // Add more channels as needed (e.g., SENSOR_CHAN_ACCEL_XYZ, SENSOR_CHAN_GYRO_XYZ, etc.)
}

impl From<c_int> for SensorChannel {
    fn from(value: c_int) -> Self {
        match value as u32 {
            raw::sensor_channel_SENSOR_CHAN_AMBIENT_TEMP => Self::AmbientTemp,
            raw::sensor_channel_SENSOR_CHAN_ALL => Self::All,
            _ => Self::All, // Default fallback
        }
    }
}

/// Sensor attribute types
///
/// Represents configurable attributes of a sensor that can be read or written.
/// These map directly to Zephyr's `sensor_attribute` enum values.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorAttribute {
    /// Sensor offset attribute
    ///
    /// Used to configure or read the offset value applied to sensor readings.
    /// This is commonly used for calibration purposes.
    ///
    /// Maps to Zephyr's `SENSOR_ATTR_OFFSET`.
    Offset = raw::sensor_attribute_SENSOR_ATTR_OFFSET as i32,
    // Add more attributes as needed (e.g., SENSOR_ATTR_SAMPLING_FREQUENCY, SENSOR_ATTR_UPPER_THRESH, etc.)
}

impl From<c_int> for SensorAttribute {
    fn from(value: c_int) -> Self {
        match value as u32 {
            raw::sensor_attribute_SENSOR_ATTR_OFFSET => Self::Offset,
            _ => Self::Offset, // Default fallback
        }
    }
}

/// Sensor value representation
///
/// Represents a sensor reading value in Zephyr's standard format.
/// The value is computed as: `val1 + val2 * 10^(-6)`
///
/// # Examples
///
/// ```
/// // Temperature of 25.5°C
/// let temp = SensorValue {
///     val1: 25,        // Integer part
///     val2: 500000,    // Fractional part (0.5 * 1,000,000)
/// };
/// // Actual value: 25 + 500000 * 0.000001 = 25.5
///
/// // Negative temperature of -10.25°C
/// let temp = SensorValue {
///     val1: -10,       // Integer part
///     val2: -250000,   // Fractional part (-0.25 * 1,000,000)
/// };
/// // Actual value: -10 + (-250000) * 0.000001 = -10.25
/// ```
///
/// # C Compatibility
///
/// This struct has the same memory layout as Zephyr's `struct sensor_value`:
/// ```c
/// struct sensor_value {
///     int32_t val1;
///     int32_t val2;
/// };
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SensorValue {
    /// Integer part of the value
    pub val1: i32,
    /// Fractional part of the value (in one-millionth parts)
    pub val2: i32,
}

impl From<raw::sensor_value> for SensorValue {
    fn from(value: raw::sensor_value) -> Self {
        Self {
            val1: value.val1,
            val2: value.val2,
        }
    }
}

impl From<SensorValue> for raw::sensor_value {
    fn from(value: SensorValue) -> Self {
        Self {
            val1: value.val1,
            val2: value.val2,
        }
    }
}

/// Trait for sensor drivers
pub trait SensorDriver: Sized {
    /// Initialize the sensor
    fn init(&mut self, dev: &Device) -> Result<(), c_int> {
        let _ = dev;
        Ok(())
    }

    /// Fetch a sample from the sensor
    fn sample_fetch(&mut self, chan: SensorChannel) -> Result<(), c_int>;

    /// Get a channel value
    fn channel_get(&self, chan: SensorChannel) -> Result<SensorValue, c_int>;

    /// Set a sensor attribute (optional)
    fn attr_set(&mut self, chan: SensorChannel, attr: SensorAttribute, val: &SensorValue) -> Result<(), c_int> {
        let _ = (chan, attr, val);
        Err(-22) // -EINVAL
    }

    /// Get a sensor attribute (optional)
    fn attr_get(&self, chan: SensorChannel, attr: SensorAttribute) -> Result<SensorValue, c_int> {
        let _ = (chan, attr);
        Err(-22) // -EINVAL
    }
}

/// Driver instance data wrapper
/// This provides type-safe access to driver-specific data
#[repr(transparent)]
pub struct DriverData<T> {
    data: *mut T,
}

impl<T> DriverData<T> {
    /// Create from raw pointer (used by generated code)
    #[doc(hidden)]
    pub const unsafe fn from_raw(data: *mut T) -> Self {
        Self { data }
    }

    /// Get a mutable reference to the driver data
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data }
    }

    /// Get a reference to the driver data
    pub fn get(&self) -> &T {
        unsafe { &*self.data }
    }
}

/// Driver configuration wrapper
/// This provides type-safe access to driver configuration from devicetree
#[repr(transparent)]
pub struct DriverConfig<T> {
    config: *const T,
}

impl<T> DriverConfig<T> {
    /// Create from raw pointer (used by generated code)
    #[doc(hidden)]
    pub const unsafe fn from_raw(config: *const T) -> Self {
        Self { config }
    }

    /// Get a reference to the driver config
    pub fn get(&self) -> &T {
        unsafe { &*self.config }
    }
}
