use std::{ffi::CString, sync::Arc};

use common::iterator::MaxOkFilterMap;
use vulkan::{Instance, Version};
use wosim_server::{DeviceCandidate, Error};

fn main() -> Result<(), Error> {
    let version = Version {
        major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        minor: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        patch: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
    };
    let instance = Arc::new(Instance::new(
        &CString::new("wosim").unwrap(),
        version,
        vec![],
    )?);
    let _device = instance
        .physical_devices()?
        .into_iter()
        .max_ok_filter_map(DeviceCandidate::new)?
        .ok_or(Error::NoSuitableDeviceFound)?
        .create()?;
    Ok(())
}
