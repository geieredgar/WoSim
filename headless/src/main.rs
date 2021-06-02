use std::{
    ffi::CString,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::vulkan::DeviceCandidate;
use ::vulkan::Instance;
use error::Error;
use net::{from_pem, local_server_address, self_signed, Server, ServerConfiguration};
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
        #[structopt(long, requires("private-key"))]
        certificate_chain: Option<PathBuf>,
        #[structopt(long)]
        private_key: Option<PathBuf>,
        #[structopt(long)]
        use_mdns: bool,
    },
    Create,
}

impl Command {
    fn run(self) -> Result<(), Error> {
        match self {
            Command::Serve {
                port,
                certificate_chain,
                private_key,
                use_mdns,
            } => {
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
                    let (certificate_chain, private_key) = if let Some(certificate_chain) =
                        certificate_chain
                    {
                        from_pem(certificate_chain, private_key.unwrap()).map_err(Error::FromPem)?
                    } else {
                        self_signed().map_err(Error::SelfSign)?
                    };
                    let mut server = Server::new(
                        service.clone(),
                        ServerConfiguration {
                            address: local_server_address(port),
                            certificate_chain,
                            private_key,
                            use_mdns,
                        },
                    );
                    server.open().map_err(Error::OpenServer)?;
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
