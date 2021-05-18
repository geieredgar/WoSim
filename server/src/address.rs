use std::sync::Arc;

use crate::Server;

pub enum ServerAddress {
    Local(Arc<Server>),
    Remote(String),
}
