//! Runtime checks corresponding to the ScreenCaptureKit bridge's macOS floor.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

use anyhow::{ensure, Context, Result};

pub(crate) fn require_supported_macos() -> Result<u32> {
    let major = macos_major_version()?;
    ensure!(major >= 13, "Screen capture requires macOS 13 or later");
    Ok(major)
}

fn macos_major_version() -> Result<u32> {
    let mut version = [0u8; 64];
    let mut length = version.len();
    // SAFETY: sysctl reads the constant NUL-terminated key and writes at most
    // length bytes into the provided buffer. No kernel value is modified.
    let result = unsafe {
        sysctlbyname(
            b"kern.osproductversion\0".as_ptr().cast(),
            version.as_mut_ptr().cast(),
            &mut length,
            ptr::null_mut(),
            0,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("Unable to determine macOS version");
    }
    ensure!(length <= version.len(), "Invalid macOS version response");
    let version = CStr::from_bytes_until_nul(&version[..length])
        .context("Invalid macOS version string")?
        .to_str()
        .context("Invalid macOS version encoding")?;
    parse_major_version(version)
}

pub(crate) fn parse_major_version(version: &str) -> Result<u32> {
    let major = version.split('.').next().context("Empty macOS version")?;
    ensure!(
        !major.is_empty() && major.bytes().all(|c| c.is_ascii_digit()),
        "Invalid macOS major version"
    );
    major.parse().context("Invalid macOS major version")
}

extern "C" {
    fn sysctlbyname(
        name: *const c_char,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *mut c_void,
        newlen: usize,
    ) -> c_int;
}
