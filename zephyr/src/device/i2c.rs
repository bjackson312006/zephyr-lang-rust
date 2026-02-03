//! I2C driver interface
//!
//! This module provides a safe Rust wrapper around Zephyr's I2C APIs
//! and implements the embedded-hal I2C traits for interoperability
//! with the embedded Rust ecosystem.

use crate::raw;
use core::ffi::c_void;
use core::ptr::NonNull;
/// I2C error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cError {
    /// Bus error (e.g., arbitration lost, clock stretch timeout)
    Bus,
    /// Invalid argument
    InvalidArgument,
    /// Device not ready
    NotReady,
    /// No ACK received
    NoAck,
    /// Other I/O error
    Io,
}

impl embedded_hal::i2c::Error for I2cError {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        match self {
            I2cError::Bus => embedded_hal::i2c::ErrorKind::Bus,
            I2cError::InvalidArgument => embedded_hal::i2c::ErrorKind::Other,
            I2cError::NotReady => embedded_hal::i2c::ErrorKind::Other,
            I2cError::NoAck => embedded_hal::i2c::ErrorKind::NoAcknowledge(
                embedded_hal::i2c::NoAcknowledgeSource::Unknown,
            ),
            I2cError::Io => embedded_hal::i2c::ErrorKind::Other,
        }
    }
}

/// I2C device wrapper
///
/// This struct provides a safe wrapper around a Zephyr I2C device
/// and implements the embedded-hal I2C traits.
pub struct I2cDevice {
    spec: NonNull<raw::i2c_dt_spec>,
}

impl I2cDevice {
    /// Create a new I2C device from a device tree spec
    ///
    /// # Safety
    /// - `spec` must point to a valid `i2c_dt_spec` structure
    /// - The spec must remain valid for the lifetime of this `I2cDevice`
    pub unsafe fn from_dt_spec(spec: *const raw::i2c_dt_spec) -> Option<Self> {
        NonNull::new(spec as *mut raw::i2c_dt_spec).map(|spec| Self { spec })
    }

    /// Check if the I2C device is ready
    pub fn is_ready(&self) -> bool {
        unsafe { raw::i2c_is_ready_dt(self.spec.as_ptr()) }
    }

    /// Get the I2C device address
    pub fn address(&self) -> u16 {
        unsafe { (*self.spec.as_ptr()).addr }
    }

    /// Write data to the I2C device
    ///
    /// # Arguments
    /// * `buf` - Buffer containing data to write
    pub fn write(&mut self, buf: &[u8]) -> Result<(), I2cError> {
        if buf.is_empty() {
            return Ok(());
        }

        let ret = unsafe { raw::i2c_write_dt(self.spec.as_ptr(), buf.as_ptr(), buf.len() as u32) };

        if ret < 0 {
            Err(Self::error_from_errno(ret))
        } else {
            Ok(())
        }
    }

    /// Read data from the I2C device
    ///
    /// # Arguments
    /// * `buf` - Buffer to store read data
    pub fn read(&mut self, buf: &mut [u8]) -> Result<(), I2cError> {
        if buf.is_empty() {
            return Ok(());
        }

        let ret =
            unsafe { raw::i2c_read_dt(self.spec.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };

        if ret < 0 {
            Err(Self::error_from_errno(ret))
        } else {
            Ok(())
        }
    }

    /// Write then read from the I2C device (write-read transaction)
    ///
    /// # Arguments
    /// * `write_buf` - Buffer containing data to write
    /// * `read_buf` - Buffer to store read data
    pub fn write_read(&mut self, write_buf: &[u8], read_buf: &mut [u8]) -> Result<(), I2cError> {
        if write_buf.is_empty() && read_buf.is_empty() {
            return Ok(());
        }

        let ret = unsafe {
            raw::i2c_write_read_dt(
                self.spec.as_ptr(),
                write_buf.as_ptr() as *const c_void,
                write_buf.len() as usize,
                read_buf.as_mut_ptr() as *mut c_void,
                read_buf.len() as usize,
            )
        };

        if ret < 0 {
            Err(Self::error_from_errno(ret))
        } else {
            Ok(())
        }
    }

    /// Convert Zephyr errno to I2cError
    fn error_from_errno(errno: i32) -> I2cError {
        // Zephyr returns negative errno values
        // Match against common error codes
        match -errno as u32 {
            zephyr_sys::EINVAL => I2cError::InvalidArgument,
            zephyr_sys::ENOTSUP => I2cError::NotReady,
            zephyr_sys::EIO => I2cError::Io,
            zephyr_sys::EBUSY => I2cError::Bus,
            _ => I2cError::Io,
        }
    }
}

// Implement embedded-hal I2C traits
impl embedded_hal::i2c::ErrorType for I2cDevice {
    type Error = I2cError;
}

impl embedded_hal::i2c::I2c for I2cDevice {
    fn read(&mut self, address: u8, read: &mut [u8]) -> Result<(), Self::Error> {
        // Note: address is ignored as it's already set in the dt_spec
        // This implementation uses the address from the device tree
        let _ = address;
        I2cDevice::read(self, read)
    }

    fn write(&mut self, address: u8, write: &[u8]) -> Result<(), Self::Error> {
        // Note: address is ignored as it's already set in the dt_spec
        let _ = address;
        I2cDevice::write(self, write)
    }

    fn write_read(
        &mut self,
        address: u8,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Self::Error> {
        // Note: address is ignored as it's already set in the dt_spec
        let _ = address;
        I2cDevice::write_read(self, write, read)
    }

    fn transaction(
        &mut self,
        _address: u8,
        _operations: &mut [embedded_hal::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        // TODO: Implement full transaction support
        Err(I2cError::InvalidArgument)
    }
}

// Additional helper for register-based devices
impl I2cDevice {
    /// Write a single byte to a register
    ///
    /// # Arguments
    /// * `reg` - Register address
    /// * `value` - Value to write
    pub fn write_register(&mut self, reg: u8, value: u8) -> Result<(), I2cError> {
        let buf = [reg, value];
        self.write(&buf)
    }

    /// Read a single byte from a register
    ///
    /// # Arguments
    /// * `reg` - Register address
    pub fn read_register(&mut self, reg: u8) -> Result<u8, I2cError> {
        let mut buf = [0u8; 1];
        self.write_read(&[reg], &mut buf)?;
        Ok(buf[0])
    }

    /// Write multiple bytes to a register
    ///
    /// # Arguments
    /// * `reg` - Register address
    /// * `data` - Data to write
    pub fn write_register_bytes(&mut self, reg: u8, data: &[u8]) -> Result<(), I2cError> {
        // Create buffer with register address + data
        let mut buf = [0u8; 33]; // Max 32 bytes + 1 register byte
        if data.len() > 32 {
            return Err(I2cError::InvalidArgument);
        }

        buf[0] = reg;
        buf[1..=data.len()].copy_from_slice(data);
        self.write(&buf[..=data.len()])
    }

    /// Read multiple bytes from a register
    ///
    /// # Arguments
    /// * `reg` - Register address
    /// * `buf` - Buffer to store read data
    pub fn read_register_bytes(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), I2cError> {
        self.write_read(&[reg], buf)
    }
}
