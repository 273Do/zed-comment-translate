//! Locating the wrapped language server binary.

use std::{
    env,
    path::{Path, PathBuf},
};

/// Resolves a bare server name, preferring the copy Zed downloaded itself
/// (the PATH entry may be a rustup shim without the component installed).
pub fn resolve(name: &str) -> PathBuf {
    let path = Path::new(name);
    if path.components().count() > 1 {
        return path.into();
    }
    zed_downloaded(name).unwrap_or_else(|| path.into())
}

fn zed_downloaded(name: &str) -> Option<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_dirs = [
        home.as_ref()
            .map(|home| home.join("Library/Application Support/Zed/languages")),
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|home| home.join(".local/share")))
            .map(|dir| dir.join("zed/languages")),
        env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("Zed/languages")),
    ];
    let binary_name = format!("{name}{}", env::consts::EXE_SUFFIX);
    data_dirs.iter().flatten().find_map(|dir| {
        let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir.join(name))
            .ok()?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let file_name = path.file_name()?.to_str()?;
                if !file_name.starts_with(name) || file_name.ends_with(".metadata") {
                    return None;
                }
                // Either the binary itself or a versioned directory containing it.
                if path.is_file() {
                    Some(path)
                } else {
                    Some(path.join(&binary_name)).filter(|p| p.is_file())
                }
            })
            .collect();
        // Versioned names like `rust-analyzer-2026-10-26` sort chronologically.
        candidates.sort();
        candidates.pop()
    })
}
