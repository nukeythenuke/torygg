use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::{config, games, modmanager};
use crate::util::verify_directory;

pub fn get_installed_mods<G>(game: &G) -> Result<Vec<String>, ToryggError> where G: games::Game {
    let mut mods = Vec::new();
    for entry in config::get_mods_dir(game).read_dir().map_err(ToryggError::IOError)? {
        let entry = entry.map_err(ToryggError::IOError)?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        mods.push(path.file_name().unwrap().to_string_lossy().to_string())
    }

    Ok(mods)
}

pub fn is_mod_installed<G>(game: &G, mod_name: &str) -> Result<bool, ToryggError> where G: games::Game {
    Ok(get_installed_mods(game)?.iter().any(|installed| installed == mod_name))
}

pub fn create_mod<G>(game: &G, mod_name: &str) -> Result<(), ToryggError> where G: games::Game {
    if is_mod_installed(game, mod_name)? {
        return Err(ToryggError::ModAlreadyExists);
    }

    verify_directory(&config::get_mods_dir(game).join(mod_name))
}

pub fn install_mod<G>(game: &G, archive: &Path, name: &str) -> Result<(), ToryggError> where G: games::Game {
    if !archive.exists() {
        return Err(ToryggError::Other("Archive does not exist!".to_owned()));
    }

    if is_mod_installed(game, name)? {
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

    // Detect if mod is contained within a subdirectory
    // and move it if it is
    let mut mod_root = archive_extract_path;
    let entries = fs::read_dir(&mod_root)
        .map_err(ToryggError::IOError)?
        .filter_map(|e| e.ok())
        .collect::<Vec<fs::DirEntry>>();
    if entries.len() == 1 {
        let path = entries[0].path();
        if path.is_dir() {
            mod_root = path
        }
    }

    // This is where we would want to handle FOMODS

    // Copy all files in the mod root to the installed mods directory
    let install_path = config::get_mods_dir(game).join(name);
    verify_directory(&install_path).unwrap();
    for entry in WalkDir::new(&mod_root)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
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

pub fn uninstall_mod<G>(game: &G, name: &str) -> Result<(), ToryggError> where G: games::Game {
    // TODO: check mod is installed

    for mut profile in crate::profile::get_profiles()? {
        profile.disable_mod(name);
    }

    let mod_dir = config::get_mods_dir(game).join(name);
    std::fs::remove_dir_all(mod_dir).map_err(ToryggError::IOError)
}
