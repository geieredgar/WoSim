use std::{io, sync::Arc};

use net::{ResolveSuccess, Verification};
use server::{create_world, AuthenticationError, CreateServiceError, Service};

pub enum Resolver {
    Create {
        token: String,
    },
    Open {
        token: String,
    },
    Remote {
        hostname: String,
        port: u16,
        token: String,
        skip_verification: bool,
    },
}

#[derive(Debug)]
pub enum ResolveError {
    CreateWorld(io::Error),
    CreateService(CreateServiceError),
    Resolve(net::ResolveError<AuthenticationError>),
}

impl Resolver {
    pub async fn resolve(self) -> Result<ResolveSuccess<Service>, ResolveError> {
        match self {
            Resolver::Create { token } => {
                create_world().map_err(ResolveError::CreateWorld)?;
                net::Resolver::Local {
                    service: Arc::new(Service::new().map_err(ResolveError::CreateService)?),
                    token,
                }
            }
            Resolver::Open { token } => net::Resolver::Local {
                service: Arc::new(Service::new().map_err(ResolveError::CreateService)?),
                token,
            },
            Resolver::Remote {
                hostname,
                port,
                token,
                skip_verification,
            } => net::Resolver::Remote {
                hostname,
                port,
                token,
                verification: if skip_verification {
                    Verification::Skip
                } else {
                    Verification::CertificateAuthorities(Vec::new())
                },
            },
        }
        .resolve()
        .await
        .map_err(ResolveError::Resolve)
    }
}
