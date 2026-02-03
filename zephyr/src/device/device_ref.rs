//! Safe wrapper around Zephyr device pointers
//!
//! This module provides a safe abstraction over raw device pointers that are passed
//! from C FFI code. The wrapper ensures pointer validity at the FFI boundary and
//! provides safe methods for common operations.

use crate::raw;
use core::ptr::NonNull;

/// A safe reference to a Zephyr device
///
/// This type wraps a raw device pointer and provides safe methods for accessing
/// device properties. The pointer validity is checked at construction time at the
/// FFI boundary, allowing the wrapper itself to be passed safely through Rust code.
#[derive(Debug, Clone, Copy)]
pub struct DeviceRef {
    ptr: NonNull<raw::device>,
}

impl DeviceRef {
    /// Create a new DeviceRef from a raw pointer
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - `ptr` is non-null and points to a valid `device` structure
    /// - The device structure remains valid for the lifetime of this reference
    /// - The device's fields (especially `config`) are properly initialized
    ///
    /// This should only be called at FFI boundaries where pointer validity
    /// can be verified (e.g., checking if device_is_ready).
    pub unsafe fn from_ptr(ptr: *const raw::device) -> Option<Self> {
        NonNull::new(ptr as *mut raw::device).map(|ptr| Self { ptr })
    }

    /// Get the raw device pointer
    ///
    /// This returns the underlying pointer for use with C APIs.
    pub fn as_ptr(&self) -> *const raw::device {
        self.ptr.as_ptr()
    }

    /// Check if the device is ready
    ///
    /// This is a safe wrapper around `device_is_ready`.
    pub fn is_ready(&self) -> bool {
        unsafe { raw::device_is_ready(self.as_ptr()) }
    }

    /// Access the device config as a reference to a specific type
    ///
    /// Returns a reference without copying the config struct.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The device's `config` field points to a valid structure of type `T`
    /// - The config structure matches the expected type for this device
    /// - The config remains valid for the returned lifetime
    ///
    /// # Example
    /// ```rust,no_run
    /// # use zephyr::device::DeviceRef;
    /// # #[repr(C)]
    /// # struct MyDeviceConfig { value: u32 }
    /// # fn example(dev: DeviceRef) {
    /// let config = unsafe { dev.config_as::<MyDeviceConfig>() };
    /// println!("Config value: {}", config.value);
    /// # }
    /// ```
    pub unsafe fn config_as<T>(&self) -> &'static T {
        &*((*self.as_ptr()).config as *const T)
    }

    /// Access the device data as a reference to a specific type
    ///
    /// Returns a reference without copying the data struct.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The device's `data` field points to a valid structure of type `T`
    /// - The data structure matches the expected type for this device
    /// - The data remains valid for the returned lifetime
    pub unsafe fn data_as<T>(&self) -> &'static T {
        &*((*self.as_ptr()).data as *const T)
    }

    /// Access the device data as a mutable specific type
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The device's `data` field points to a valid structure of type `T`
    /// - The data structure matches the expected type for this device
    /// - The data remains valid for the returned lifetime
    /// - No other references to the data exist
    pub unsafe fn data_as_mut<T>(&self) -> &'static mut T {
        &mut *((*self.as_ptr()).data as *mut T)
    }
}
