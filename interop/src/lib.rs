use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplicationInfo {
    pub name: String,
    pub protocol: String,
    pub format_req: WorldFormatReq,
    pub format: WorldFormat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldFormat {
    pub base: String,
    pub version: Version,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldFormatReq {
    pub base: String,
    pub version: VersionReq,
}

impl WorldFormatReq {
    pub fn matches(&self, format: &WorldFormat) -> bool {
        self.base == format.base && self.version.matches(&format.version)
    }
}
