use crate::{installation::Installation, server::Server, world::World};

#[derive(Clone, Debug)]
pub enum Message {
    Show,
    SelectWorldTab,
    SelectServerTab,
    ScanLocalServers,
    SetupCreateWorld,
    SetupDeleteWorld(World),
    DeleteWorld(World),
    PlayWorld(World),
    JoinServer(Server),
    FoundLocalServer(Server),
    ChangeName(String),
    SelectVersion(Installation),
    CreateWorld(String, Installation),
}
