use std::time::Duration;

#[derive(Debug)]
pub enum Connection {
    Local,
    Remote(quinn::Connection),
}

impl Connection {
    pub fn rtt(&self) -> Duration {
        match self {
            Connection::Local => Duration::from_secs(0),
            Connection::Remote(connection) => connection.rtt(),
        }
    }
}
