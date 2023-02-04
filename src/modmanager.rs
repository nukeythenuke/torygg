use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::{config, games};
use crate::util::verify_directory;

pub fn create_mod<G>(game: &G, mod_name: &str) -> Result<(), ToryggError> where G: games::Game {
    // TODO: check mod name is not already used (installed already)
    // Err(ToryggError::ModAlreadyExists)

    verify_directory(&config::get_mods_dir(game).join(mod_name))
}

pub fn install_mod<G>(game: &G, archive: &Path, name: &str) -> Result<(), &'static str> where G: games::Game {
    if !archive.exists() {
        return Err("Archive does not exist!");
    }

    // TODO: check mod name is not already used (installed already)

    let archive_extract_dir = TempDir::new().unwrap();
    let archive_extract_path = archive_extract_dir.into_path();

    // Use p7zip to extract the archive to a temporary directory
    let mut command = Command::new("7z");
    command.arg("x");
    command.arg(format!("-o{}", archive_extract_path.display()));
    command.arg(archive);

    let status = command.status().map_err(|_| "Unable to extract archive")?;
    if !status.success() {
        return Err("Unable to extract archive");
    }

    // Detect if mod is contained within a subdirectory
    // and move it if it is
    let mut mod_root = archive_extract_path;
    let entries = fs::read_dir(&mod_root)
        .map_err(|_| "Couldn't read dir")?
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

pub fn uninstall_mod<G>(game: &G, name: &str) -> Result<(), &'static str> where G: games::Game {
    todo!()
    // Remove mod from all profiles
    // Delete mod files
}