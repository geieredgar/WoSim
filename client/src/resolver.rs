use std::{io, sync::Arc};

use net::{ResolveSuccess, Verification};
use server::{create_world, AuthenticationError, CreateServiceError, Service};
use thiserror::Error;

pub enum Resolver {
    Create {
        token: String,
        port: u16,
    },
    Open {
        token: String,
        port: u16,
    },
    Remote {
        hostname: String,
        port: u16,
        token: String,
        skip_verification: bool,
    },
}

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("could not create world")]
    CreateWorld(#[source] io::Error),
    #[error("could not create service")]
    CreateService(#[source] CreateServiceError),
    #[error(transparent)]
    Resolve(#[from] net::ResolveError<AuthenticationError>),
}

impl Resolver {
    pub async fn resolve(self) -> Result<ResolveSuccess<Service>, ResolveError> {
        Ok(match self {
            Resolver::Create { token, port } => {
                create_world().map_err(ResolveError::CreateWorld)?;
                net::Resolver::Local {
                    service: Arc::new(Service::new().map_err(ResolveError::CreateService)?),
                    token,
                    port,
                }
            }
            Resolver::Open { token, port } => net::Resolver::Local {
                service: Arc::new(Service::new().map_err(ResolveError::CreateService)?),
                token,
                port,
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
        .await?)
    }
}
