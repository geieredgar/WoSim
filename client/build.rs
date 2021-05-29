use build::{new_shader_compiler, CompilerExt};

fn main() -> eyre::Result<()> {
    stable_eyre::install()?;
    let mut compiler = new_shader_compiler()?;
    compiler.compile("cull.comp")?;
    compiler.compile("depth_pyramid.comp")?;
    compiler.compile("egui.frag")?;
    compiler.compile("egui.vert")?;
    compiler.compile("scene.frag")?;
    compiler.compile("scene.vert")?;
    Ok(())
}
