use wosim_common::vulkan::{Image, ImageView};

pub(super) struct Font {
    pub view: ImageView,
    pub _image: Image,
    pub version: u64,
}
