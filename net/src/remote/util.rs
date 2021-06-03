use std::{
    fs::read,
    io,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
};

use quinn::{Certificate, CertificateChain, ParseError, PrivateKey};
use rcgen::{generate_simple_self_signed, RcgenError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SelfSignError {
    #[error("could not generate certificates")]
    GenerateCertificates(#[source] RcgenError),
    #[error("could not parse private key")]
    ParsePrivateKey(#[source] ParseError),
    #[error("could not serialize certificate")]
    SerializeCertificate(#[source] RcgenError),
    #[error("could not parse certificate")]
    ParseCertificate(#[source] ParseError),
}

#[derive(Debug, Error)]
pub enum FromPemError {
    #[error("could not read certificate chain file '{1}'")]
    ReadCertificateChain(#[source] io::Error, PathBuf),
    #[error("could not parse certificate chain")]
    ParseCertificateChain(#[source] ParseError),
    #[error("could not read private key file '{1}'")]
    ReadPrivateKey(#[source] io::Error, PathBuf),
    #[error("could not parse private key")]
    ParsePrivateKey(#[source] ParseError),
}

pub fn self_signed() -> Result<(CertificateChain, PrivateKey), SelfSignError> {
    let cert = generate_simple_self_signed(["localhost".to_owned()])
        .map_err(SelfSignError::GenerateCertificates)?;
    let der = cert.serialize_private_key_der();
    let private_key = PrivateKey::from_der(&der).map_err(SelfSignError::ParsePrivateKey)?;
    let der = cert
        .serialize_der()
        .map_err(SelfSignError::SerializeCertificate)?;
    let cert = Certificate::from_der(&der).map_err(SelfSignError::ParseCertificate)?;
    let certificate_chain = CertificateChain::from_certs(vec![cert]);
    Ok((certificate_chain, private_key))
}

pub fn from_pem(
    certificate_chain: impl AsRef<Path>,
    private_key: impl AsRef<Path>,
) -> Result<(CertificateChain, PrivateKey), FromPemError> {
    let certificate_chain = read(certificate_chain.as_ref()).map_err(|e| {
        FromPemError::ReadCertificateChain(e, certificate_chain.as_ref().to_path_buf())
    })?;
    let certificate_chain = CertificateChain::from_pem(&certificate_chain)
        .map_err(FromPemError::ParseCertificateChain)?;
    let private_key = read(private_key.as_ref())
        .map_err(|e| FromPemError::ReadPrivateKey(e, private_key.as_ref().to_path_buf()))?;
    let private_key = PrivateKey::from_pem(&private_key).map_err(FromPemError::ParsePrivateKey)?;
    Ok((certificate_chain, private_key))
}

#[cfg(windows)]
pub fn local_server_address(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
}

#[cfg(not(windows))]
pub fn local_server_address(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
}
