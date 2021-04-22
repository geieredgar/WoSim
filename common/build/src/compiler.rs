use std::{
    env::var,
    fs::{create_dir_all, read_to_string, write},
    path::Path,
};

use shaderc::{CompileOptions, Compiler, IncludeType, ResolvedInclude, ShaderKind};

use crate::Error;

pub fn new_shader_compiler() -> Result<Compiler, Error> {
    Compiler::new().ok_or(Error::MissingCompiler)
}

pub trait CompilerExt {
    fn compile(&mut self, path: &str) -> Result<(), Error>;
}

impl CompilerExt for Compiler {
    fn compile(&mut self, path: &str) -> Result<(), Error> {
        let input_path = Path::new("shaders").join(path);
        let output_path = Path::new(&var("OUT_DIR")?).join(format!("{}.spv", path));
        println!("cargo:rerun-if-changed={}", input_path.to_str().unwrap());
        let source_text = read_to_string(&input_path)?;
        let mut compile_options = CompileOptions::new().unwrap();
        compile_options.set_generate_debug_info();
        compile_options.set_include_callback(include_callback);
        let artifact = self.compile_into_spirv(
            &source_text,
            ShaderKind::InferFromSource,
            input_path.to_str().unwrap(),
            "main",
            Some(&compile_options),
        )?;
        print_warnings(&artifact.get_warning_messages());
        create_dir_all(output_path.parent().unwrap())?;
        write(output_path, artifact.as_binary_u8())?;
        Ok(())
    }
}

fn include_callback(
    name: &str,
    include_type: IncludeType,
    source: &str,
    _depth: usize,
) -> Result<ResolvedInclude, String> {
    let path = match include_type {
        IncludeType::Relative => {
            let source_path = Path::new(source);
            let parent = source_path.parent().unwrap();
            parent.join(Path::new(name))
        }
        IncludeType::Standard => Path::new("shaders").join(name),
    };
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
    match read_to_string(&path) {
        Ok(content) => Ok(ResolvedInclude {
            content,
            resolved_name: path.to_str().unwrap().into(),
        }),
        Err(error) => Err(format!(
            "Could not include {} from {}: {}",
            name, source, error
        )),
    }
}

fn print_warnings(text: &str) {
    text.lines()
        .for_each(|line| println!("cargo:warning={}", line));
}
