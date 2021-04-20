use ash::vk::make_version;

#[derive(Clone, Copy)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl From<Version> for u32 {
    fn from(v: Version) -> Self {
        make_version(v.major, v.minor, v.patch)
    }
}
