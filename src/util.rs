use crate::error::ToryggError;
use std::path::Path;
use std::{fs, fs::File, path::PathBuf};
use crate::games::SteamApp;

#[must_use]
pub fn libraryfolders_vdf() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap()).join(".steam/root/config/libraryfolders.vdf")
}

pub fn steam_library(app: &SteamApp) -> Result<PathBuf, ToryggError> {
    let vdf = libraryfolders_vdf();
    let mut file = File::open(vdf)?;
    let kvs = torygg_vdf::parse(&mut file)?;

    for kv in &kvs {
        let components = kv.0.iter().collect::<Vec<_>>();
        // Key we want:                    🠗
        // libraryfolders/<lib_id>/apps/<appid>
        if let Some(component) = components.get(3) {
            if *component == app.appid().to_string().as_str() {
                // libraryfolders/<lib_id>/path
                let path = kv.0.iter().take(2).collect::<PathBuf>().join("path");

                return Ok(kvs[&path].clone().into());
            }
        }
    }

    Err(ToryggError::SteamLibraryNotFound)
}

pub fn verify_directory(path: &Path) -> Result<(), ToryggError> {
    if path.exists() {
        return if path.is_dir() {
            Ok(())
        } else {
            Err(ToryggError::NotADirectory(path.to_owned()))
        };
    }

    fs::create_dir(path)?;
    Ok(())
}
