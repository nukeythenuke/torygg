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

pub fn find_case_insensitive_path(root: &Path, relative: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in relative.components() {
        let mut path_exists = true;
        if path_exists {
            let mut found = false;
            for entry in fs::read_dir(root.join(&result)).unwrap() {
                let entry = entry.unwrap();
                let file_name = entry.file_name();
                if unicase::eq(&file_name.to_string_lossy(), &component.as_os_str().to_string_lossy()) {
                    result.push(file_name);
                    found = true;
                    break;
                }
            }

            path_exists = found;
            if !path_exists {
                result.push(component);
            }
        } else {
            result.push(component);
        }
    }

    result
}
