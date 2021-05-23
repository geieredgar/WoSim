use interop::WorldFormat;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct World {
    pub path: PathBuf,
    pub info: WorldInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldInfo {
    pub format: WorldFormat,
}
