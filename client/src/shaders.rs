use util::{asset::AssetLoader, include_shader};

pub const CULL_COMP: AssetLoader = include_shader!("cull.comp");
pub const DEPTH_PYRAMID_COMP: AssetLoader = include_shader!("depth_pyramid.comp");
pub const EGUI_FRAG: AssetLoader = include_shader!("egui.frag");
pub const EGUI_VERT: AssetLoader = include_shader!("egui.vert");
pub const SCENE_FRAG: AssetLoader = include_shader!("scene.frag");
pub const SCENE_VERT: AssetLoader = include_shader!("scene.vert");
