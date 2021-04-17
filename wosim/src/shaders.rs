use wosim_common::{asset::AssetLoader, include_shader};

pub const DEFAULT_VERT: AssetLoader = include_shader!("default.vert");
pub const DEFAULT_FRAG: AssetLoader = include_shader!("default.frag");
pub const EGUI_VERT: AssetLoader = include_shader!("egui.vert");
pub const EGUI_FRAG: AssetLoader = include_shader!("egui.frag");
