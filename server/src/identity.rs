use actor::Address;

use crate::ClientMessage;

#[derive(Clone)]
pub(super) struct Identity {
    pub(super) name: String,
    pub(super) address: Address<ClientMessage>,
}
