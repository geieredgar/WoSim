use crate::Server;

pub enum ServerAddress<'a> {
    Local(&'a Server),
    Remote(String),
}
