use std::time::Duration;

use crate::{Server, Service};

pub enum Connection<S: Service> {
    Local(Server<S>),
    Remote(quinn::Connection),
}

impl<S: Service> Connection<S> {
    pub fn rtt(&self) -> Duration {
        match self {
            Connection::Local(_) => Duration::from_secs(0),
            Connection::Remote(connection) => connection.rtt(),
        }
    }
}
