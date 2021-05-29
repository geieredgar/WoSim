use build::{new_shader_compiler, CompilerExt};
use eyre::Context;

fn main() -> eyre::Result<()> {
    stable_eyre::install()?;
    let mut compiler = new_shader_compiler()?;
    compiler
        .compile("cull.comp")
        .wrap_err("could not compile cull.comp")?;
    compiler
        .compile("depth_pyramid.comp")
        .wrap_err("could not compile depth_pyramid.comp")?;
    compiler
        .compile("egui.frag")
        .wrap_err("could not compile egui.frag")?;
    compiler
        .compile("egui.vert")
        .wrap_err("could not compile egui.vert")?;
    compiler
        .compile("scene.frag")
        .wrap_err("could not compile scene.frag")?;
    compiler
        .compile("scene.vert")
        .wrap_err("could not compile scene.vert")?;
    Ok(())
}
