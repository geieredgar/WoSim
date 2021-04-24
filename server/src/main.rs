use std::{
    ffi::CString,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::{runtime::Runtime, time::sleep};
use util::iterator::MaxOkFilterMap;
use vulkan::{Instance, Version};
use wosim_server::{DeviceCandidate, Error, Server};

fn main() -> Result<(), Error> {
    env_logger::init();
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .unwrap();
    let runtime = Runtime::new()?;
    runtime.block_on(async {
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
        let mut server = Server::new();
        server.open(&"0.0.0.0:8888".parse()?)?;
        while running.load(Ordering::SeqCst) {
            sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    })
}
