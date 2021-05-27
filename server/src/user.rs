use net::Connection;
use uuid::Uuid;

use crate::Push;

#[derive(Clone, Debug)]
pub(super) struct User {
    pub(super) uuid: Uuid,
    pub(super) name: String,
    pub(super) connection: Connection<Push>,
}
