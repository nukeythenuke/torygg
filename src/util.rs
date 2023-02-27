use crate::error::ToryggError;
use crate::games;
use std::path::Path;
use std::{fs, fs::File, iter::FromIterator, path::PathBuf};

pub fn libraryfolders_vdf() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap()).join(".steam/root/config/libraryfolders.vdf")
}

pub fn steam_library(app: &games::SteamApp) -> Result<PathBuf, ToryggError> {
    let vdf = libraryfolders_vdf();
    let mut file = File::open(vdf)?;
    let kvs = torygg_vdf::parse(&mut file)?;

    for kv in &kvs {
        let components = kv.0.iter().collect::<Vec<_>>();
        // Key we want:                    🠗
        // libraryfolders/<lib_id>/apps/<appid>
        if let Some(component) = components.get(3) {
            if *component == app.appid.to_string().as_str() {
                // libraryfolders/<lib_id>/path
                let path = PathBuf::from_iter(kv.0.iter().take(2)).join("path");

                return Ok(kvs[&path].clone().into());
            }
        }
    }

    Err(ToryggError::SteamLibraryNotFound)
}

pub fn verify_directory(path: &Path) -> Result<(), ToryggError> {
    if path.exists() {
        return if !path.is_dir() {
            Err(ToryggError::NotADirectory(path.to_owned()))
        } else {
            Ok(())
        };
    }

    fs::create_dir(path)?;
    Ok(())
}
