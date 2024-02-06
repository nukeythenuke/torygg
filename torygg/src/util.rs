use crate::error::ToryggError;
use std::path::Path;
use std::{fs::File, path::PathBuf};
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

pub fn find_case_insensitive_path<P1: AsRef<Path>, P2: AsRef<Path>>(root: P1, relative: P2) -> PathBuf {
    let root = root.as_ref();
    let relative = relative.as_ref();

    let mut result = PathBuf::new();
    let components = relative.components();
    let mut path_exists = true;
    for component in components {
        if !path_exists {
            result.push(component);
            continue;
        }

        let mut found = false;
        for entry in root.join(&result).read_dir().unwrap().filter_map(Result::ok) {
            let file_name = entry.file_name();
            if unicase::eq(&file_name.to_string_lossy(), &component.as_os_str().to_string_lossy()) {
                result.push(file_name);
                found = true;
                break;
            }
        }

        if !found {
            path_exists = false;
            result.push(component);
        }
    }

    result
}
