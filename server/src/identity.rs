use actor::Address;

use crate::Push;

#[derive(Clone, Debug)]
pub(super) struct Identity {
    pub(super) name: String,
    pub(super) address: Address<Push>,
}
