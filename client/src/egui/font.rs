use wosim_common_vulkan::{Image, ImageView};

pub(super) struct Font {
    pub view: ImageView,
    pub _image: Image,
    pub version: u64,
}
