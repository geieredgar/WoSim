use ash::vk::make_version;
use semver::Version;
pub trait VersionExt {
    fn to_u32(&self) -> u32;
}

impl VersionExt for Version {
    fn to_u32(&self) -> u32 {
        make_version(self.major as u32, self.minor as u32, self.patch as u32)
    }
}
