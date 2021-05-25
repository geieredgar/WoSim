use net::Connection;

use crate::Push;

#[derive(Clone, Debug)]
pub(super) struct Identity {
    pub(super) name: String,
    pub(super) connection: Connection<Push>,
}
