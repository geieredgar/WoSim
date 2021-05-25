use std::error::Error;
use std::fmt::Debug;

use actor::Address;
use quinn::{CertificateChain, PrivateKey, TransportConfig};

use crate::{AuthToken, Connection, Message};

pub trait Service: Send + Sync + 'static {
    type AuthError: Error + Send;
    type Push: Message + Debug;
    type Request: Message + Debug;

    fn authenticate(
        &self,
        connection: Connection<Self::Push>,
        token: AuthToken,
    ) -> Result<Address<Self::Request>, Self::AuthError>;

    fn token_size_limit(&self) -> usize;

    fn service_type() -> &'static str;

    fn protocol() -> &'static str;

    fn client_transport_config() -> TransportConfig {
        TransportConfig::default()
    }

    fn server_transport_config() -> TransportConfig {
        TransportConfig::default()
    }

    fn authentication_type(&self) -> &str;

    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn certificate_chain(&self) -> CertificateChain;

    fn private_key(&self) -> PrivateKey;
}
