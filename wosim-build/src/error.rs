use std::{env::VarError, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Var(VarError),
    ShaderC(shaderc::Error),
    MissingCompiler,
}

impl From<VarError> for Error {
    fn from(error: VarError) -> Self {
        Self::Var(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<shaderc::Error> for Error {
    fn from(error: shaderc::Error) -> Self {
        Self::ShaderC(error)
    }
}
