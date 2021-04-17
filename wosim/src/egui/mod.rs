use wosim_common::vulkan::{Image, ImageView};

mod context;
mod frame;
mod view;

pub use context::*;
pub use frame::*;
pub use view::*;

struct Font {
    view: ImageView,
    _image: Image,
    version: u64,
}
