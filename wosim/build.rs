use wosim_build::{new_shader_compiler, CompilerExt, Error};

fn main() -> Result<(), Error> {
    let mut compiler = new_shader_compiler()?;
    compiler.compile("default.vert")?;
    compiler.compile("default.frag")?;
    Ok(())
}
