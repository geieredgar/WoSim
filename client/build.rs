use wosim_common_build::{new_shader_compiler, CompilerExt, Error};

fn main() -> Result<(), Error> {
    let mut compiler = new_shader_compiler()?;
    compiler.compile("cull.comp")?;
    compiler.compile("depth_pyramid.comp")?;
    compiler.compile("egui.frag")?;
    compiler.compile("egui.vert")?;
    compiler.compile("scene.frag")?;
    compiler.compile("scene.vert")?;
    Ok(())
}
