use std::{
    env::{consts::EXE_SUFFIX, current_dir},
    fs::canonicalize,
    io,
    path::{Path, PathBuf},
    process::Command,
};

use interop::ApplicationInfo;
use serde_json::from_slice;

use crate::{server::Server, world::World};

#[derive(Clone, Debug)]
pub struct Installation {
    pub path: PathBuf,
    pub info: ApplicationInfo,
}

impl Installation {
    pub fn program_name() -> String {
        format!("wosim{}", EXE_SUFFIX)
    }

    pub fn try_from(path: PathBuf) -> io::Result<Self> {
        let path = canonicalize(path)?;
        let output = Command::new(path.as_os_str()).arg("info").output()?;
        let info = from_slice(&output.stdout)?;
        Ok(Self { path, info })
    }

    pub fn scan_current_dir() -> Vec<Installation> {
        let mut installations = Vec::new();
        let program_name = Self::program_name();
        if let Ok(path) = current_dir() {
            let program = path.join(&program_name);
            if let Ok(installation) = Installation::try_from(program) {
                installations.push(installation)
            }
        }
        if let Ok(dirs) = Path::new("target").read_dir() {
            for entry in dirs {
                let entry = if let Ok(entry) = entry {
                    entry
                } else {
                    continue;
                };
                if let Ok(installation) = Installation::try_from(entry.path().join(&program_name)) {
                    installations.push(installation)
                }
            }
        }
        installations
    }

    pub fn supports_world(&self, world: &World) -> bool {
        self.info.format_req.matches(&world.info.format)
    }

    pub fn supports_server(&self, server: &Server) -> bool {
        self.info.protocol == server.protocol
    }
}
