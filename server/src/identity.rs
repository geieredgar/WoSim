use actor::Address;

use crate::ClientMessage;

#[derive(Clone, Debug)]
pub(super) struct Identity {
    pub(super) name: String,
    pub(super) address: Address<ClientMessage>,
}
