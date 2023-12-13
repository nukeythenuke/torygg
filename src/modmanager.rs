use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::{config, games};
use crate::util::verify_directory;

/// Get a vec of all installed mods for the given game
///
/// # Errors
/// Errors when the mod directory cannot be read
///
/// # Panics
/// Panics when a mods name cannot be determined from its path
pub fn installed_mods<G>(game: &G) -> Result<Vec<String>, ToryggError> where G: games::Game {
    let mut mods = Vec::new();
    for entry in config::mods_dir(game).read_dir().map_err(ToryggError::IOError)? {
        let entry = entry.map_err(ToryggError::IOError)?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        mods.push(path.file_name().unwrap().to_string_lossy().to_string());
    }

    Ok(mods)
}

/// Check if a mod exists for the given game
///
/// # Errors
/// Errors when installed mods cannot be retrieved
pub fn mod_installed<G>(game: &G, mod_name: &str) -> Result<bool, ToryggError> where G: games::Game {
    Ok(installed_mods(game)?.iter().any(|installed| installed == mod_name))
}

/// Create a new mod with the given name for the given game
///
/// # Errors
/// Errors when a mod of the same name is already installed
pub fn create_mod<G>(game: &G, mod_name: &str) -> Result<(), ToryggError> where G: games::Game {
    if mod_installed(game, mod_name)? {
        return Err(ToryggError::ModAlreadyExists);
    }

    verify_directory(&config::mods_dir(game).join(mod_name))
}

/// Install a mod for the given game
///
/// # Errors
///  - The archive path does not exist
///  - A mod of the same name already exists
///  - The status of 7z cannot be gotten
///  - 7z returns unsuccessfully
///  - The extracted mods directory cannot be read
///
/// # Panics
///  - A temporary directory cannot be created
///  - Mod directory cannot be created
///  - Copying from temp to final directory fails
pub fn install_mod<G>(game: &G, archive: &Path, name: &str) -> Result<(), ToryggError> where G: games::Game {
    if !archive.exists() {
        return Err(ToryggError::Other("Archive does not exist!".to_owned()));
    }

    if mod_installed(game, name)? {
        return Err(ToryggError::ModAlreadyExists)
    }

    let archive_extract_dir = TempDir::new().unwrap();
    let archive_extract_path = archive_extract_dir.into_path();

    // Use p7zip to extract the archive to a temporary directory
    let mut command = Command::new("7z");
    command.arg("x");
    command.arg(format!("-o{}", archive_extract_path.display()));
    command.arg(archive);

    let status = command.status().map_err(ToryggError::IOError)?;
    if !status.success() {
        return Err(ToryggError::Other("Unable to extract archive".to_owned()));
    }

    // TODO: this is broken, some mods that have one directory eg. 'SKSE' it should not be lowered
    // TODO: maybe only lower if the folder name is 'Data' or the name of the archive
    // TODO: we may need to handle both eg. 'mod_name/Data/actual_mod_stuff'
    // Detect if mod is contained within a subdirectory
    // and move it if it is
    let mut mod_root = archive_extract_path;
    let entries = fs::read_dir(&mod_root)
        .map_err(ToryggError::IOError)?
        .filter_map(Result::ok)
        .collect::<Vec<fs::DirEntry>>();
    if entries.len() == 1 {
        let path = entries[0].path();
        if path.is_dir() {
            mod_root = path;
        }
    }

    // This is where we would want to handle FOMODS

    // Copy all files in the mod root to the installed mods directory
    let install_path = config::mods_dir(game).join(name);
    verify_directory(&install_path).unwrap();
    for entry in WalkDir::new(&mod_root)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
    {
        let from = entry.path();
        let relative_path = from.strip_prefix(&mod_root).unwrap();
        let to = install_path.join(relative_path);

        if from.is_dir() {
            fs::create_dir(to).unwrap();
        } else {
            fs::copy(from, to).unwrap();
        }
    }

    Ok(())
}

/// Uninstall a mod for the given game and disables the mod in all profiles
///
/// # Errors
///  - Profiles cannot be gotten
///  - Removing the files fails
pub fn uninstall_mod<G>(game: &G, name: &str) -> Result<(), ToryggError> where G: games::Game {
    // TODO: check mod is installed

    for mut profile in crate::profile::profiles()? {
        profile.disable_mod(name);
    }

    let mod_dir = config::mods_dir(game).join(name);
    fs::remove_dir_all(mod_dir).map_err(ToryggError::IOError)
}
