//! Device wrappers for temperature sensor.

use super::{NoStatic, Unique};
use crate::raw;
use log::info;

/// A flash controller
///
/// This is a wrapper around the `struct device` in Zephyr that represents a temperature sensor.
#[allow(dead_code)]
pub struct TemperatureSensor {
    pub(crate) device: *const raw::device,
}

#[doc = " @brief Representation of a sensor readout value.\n\n The value is represented as having an integer and a fractional part,\n and can be obtained using the formula val1 + val2 * 10^(-6). Negative\n values also adhere to the above formula, but may need special attention.\n Here are some examples of the value representation:\n\n      0.5: val1 =  0, val2 =  500000\n     -0.5: val1 =  0, val2 = -500000\n     -1.0: val1 = -1, val2 =  0\n     -1.5: val1 = -1, val2 = -500000"]
pub struct TemperatureValue {
    #[doc = " Integer part of the value."]
    pub val1: i32,
    #[doc = " Fractional part of the value (in one-millionth parts)."]
    pub val2: i32,
}

impl TemperatureSensor {
    /// Constructor, intended to be called by devicetree generated code.
    #[allow(dead_code)]
    pub(crate) unsafe fn new(
        unique: &Unique,
        _static: &NoStatic,
        device: *const raw::device,
    ) -> Option<TemperatureSensor> {
        if !unique.once() {
            return None;
        }

        Some(TemperatureSensor { device })
    }

    #[doc = " @brief Get the value of the sensor.\n\n This function reads the sensor and returns the value.\n\n @return The value of the sensor, or a negative error code if the\n         sensor could not be read."]
    pub fn read_ambient_temperature(&self) -> Result<TemperatureValue, ::core::ffi::c_int> {
        unsafe {
            let mut temp: raw::sensor_value = Default::default();
            let ret = raw::sensor_attr_set(
                self.device,
                raw::sensor_channel_SENSOR_CHAN_AMBIENT_TEMP,
                raw::sensor_attribute_SENSOR_ATTR_OFFSET,
                &mut temp,
            );
            if ret != 0 {
                info!("Fail sensor_attr_set {}", ret);
            }

            let mut temp: raw::sensor_value = Default::default();
            let ret = raw::sensor_sample_fetch(self.device);
            if ret != 0 {
                info!("Fail sensor_sample_fetch");
                return Err(ret);
            }

            let ret = raw::sensor_channel_get(
                self.device,
                raw::sensor_channel_SENSOR_CHAN_AMBIENT_TEMP,
                &mut temp,
            );
            if ret != 0 {
                info!("Fail sensor_channel_get");
                return Err(ret);
            }

            Ok(TemperatureValue {
                val1: temp.val1,
                val2: temp.val2,
            })
        }
    }
}
