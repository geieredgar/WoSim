use std::{
    fs::File,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde_json::from_reader;

pub fn scan_dir<T: DeserializeOwned, E>(
    path: impl AsRef<Path>,
    f: impl Fn(PathBuf, T) -> E,
) -> Vec<E> {
    let mut entries = Vec::new();
    if let Ok(dirs) = path.as_ref().read_dir() {
        for entry in dirs {
            let entry = if let Ok(entry) = entry {
                entry
            } else {
                continue;
            };
            if let Ok(file_type) = entry.file_type() {
                if !file_type.is_dir() {
                    continue;
                }
            } else {
                continue;
            }
            let path = entry.path();
            let info = path.join("info.json");
            let file = if let Ok(file) = File::open(info) {
                file
            } else {
                continue;
            };
            if let Ok(info) = from_reader(file) {
                entries.push(f(path, info))
            }
        }
    }
    entries
}
