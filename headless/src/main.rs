use std::{
    ffi::CString,
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::vulkan::DeviceCandidate;
use ::vulkan::Instance;
use error::Error;
use net::Server;
use semver::Version;
use server::{create_world, Service};
use structopt::StructOpt;
use tokio::{runtime::Runtime, time::sleep};
use util::iterator::MaxOkFilterMap;

mod error;
mod vulkan;

#[derive(StructOpt)]
enum Command {
    Serve {
        #[structopt(long, short, default_value = "2021")]
        port: u16,
    },
    Create,
}

impl Command {
    fn run(self) -> Result<(), Error> {
        match self {
            Command::Serve { port } => {
                let running = Arc::new(AtomicBool::new(true));
                let r = running.clone();
                ctrlc::set_handler(move || {
                    r.store(false, Ordering::SeqCst);
                })
                .unwrap();
                let runtime = Runtime::new()?;
                runtime.block_on(async {
                    let version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
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
                    let service = Arc::new(Service::new().map_err(Error::CreateService)?);
                    let mut server = Server::new(
                        service.clone(),
                        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0)),
                    );
                    server.open(false).map_err(Error::OpenServer)?;
                    while running.load(Ordering::SeqCst) {
                        sleep(Duration::from_millis(10)).await;
                    }
                    server.close();
                    let _ = service.stop().await;
                    Ok(())
                })
            }
            Command::Create => create_world().map_err(Error::Io),
        }
    }
}

fn main() -> Result<(), Error> {
    env_logger::init();
    Command::from_args().run()
}
