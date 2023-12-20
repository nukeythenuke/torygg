use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use log::info;
use tempfile::TempDir;
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::{config, fomod, Torygg};
use crate::fomod::FomodCallback;
use crate::util::verify_directory;

/// Get a vec of all installed mods for the given game
///
/// # Errors
/// Errors when the mod directory cannot be read
///
/// # Panics
/// Panics when a mods name cannot be determined from its path
pub fn installed_mods() -> Result<Vec<String>, ToryggError>  {
    let mut mods = Vec::new();
    for entry in config::mods_dir().read_dir().map_err(ToryggError::IOError)? {
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
pub fn mod_installed(mod_name: &String) -> Result<bool, ToryggError> {
    Ok(installed_mods()?.contains(mod_name))
}

/// Create a new mod with the given name for the given game
///
/// # Errors
/// Errors when a mod of the same name is already installed
pub fn create_mod(mod_name: &String) -> Result<(), ToryggError> {
    if mod_installed(mod_name)? {
        return Err(ToryggError::ModAlreadyExists);
    }

    verify_directory(&config::mods_dir().join(mod_name))
}

fn extract_archive(archive: &Path) -> Result<TempDir, ToryggError> {
    let archive_extract_dir = TempDir::new().unwrap();
    let archive_extract_path = archive_extract_dir.path();

    // Use p7zip to extract the archive to a temporary directory
    let mut command = Command::new("7z");
    command.arg("x");
    command.arg(format!("-o{}", archive_extract_path.display()));
    command.arg(archive);

    let status = command.stdout(Stdio::null()).status()?;
    if !status.success() {
        return Err(ToryggError::Other("Unable to extract archive".to_owned()));
    }

    Ok(archive_extract_dir)
}

pub(crate) fn install_all(mod_root: &Path, name: &String) -> Result<(), ToryggError> {
    let install_path = config::mods_dir().join(name);
    verify_directory(&install_path)?;

    let entries = WalkDir::new(mod_root)
        .min_depth(1).into_iter()
        .filter_map(Result::ok);

    for entry in entries {
        let from = entry.path();
        let relative_path = from.strip_prefix(mod_root).unwrap();
        let to = install_path.join(relative_path);

        if from.is_dir() {
            fs::create_dir(to)?;
        } else {
            fs::copy(from, to)?;
        }
    }

    Ok(())
}

/// Install a mod
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
pub fn install_mod(archive: &Path, name: &String, fomod_callback: FomodCallback) -> Result<(), ToryggError> {
    if !archive.exists() {
        return Err(ToryggError::Other("Archive does not exist!".to_owned()));
    }

    if mod_installed(name)? {
        return Err(ToryggError::ModAlreadyExists)
    }

    let archive_extract_path = extract_archive(archive)?;

    // lower the `mod_root` if the folder name is 'Data' or the name of the archive
    // we may need to handle both eg. 'mod_name/Data/actual_mod_stuff'
    let archive_stem = archive.file_stem().unwrap();
    let mut mod_root = archive_extract_path.path().to_owned();
    loop {
        let entries = fs::read_dir(&mod_root)
            .map_err(ToryggError::IOError)?
            .filter_map(Result::ok)
            .collect::<Vec<fs::DirEntry>>();
        if entries.len() == 1 {
            let entry = &entries[0];
            let file_name = entry.file_name();
            let path = entry.path();

            let is_archive_name = unicase::eq(&file_name.to_string_lossy(), &archive_stem.to_string_lossy());
            let is_data = unicase::eq(&file_name.to_string_lossy(), &OsStr::new("Data").to_string_lossy());
            if path.is_dir() &&  (is_archive_name || is_data) {
                mod_root = path;
            }
        } else {
            break
        }
    }

    let entries = fs::read_dir(&mod_root).unwrap()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    for entry in entries {
        if unicase::eq(entry.file_name().to_string_lossy().as_ref(), "fomod"){
            info!("found fomod");
            return fomod::fomod_install(&mod_root, &entry.path(), name, fomod_callback);
        }
    }

    install_all(&mod_root, name)
}

/// Uninstall a mod for the given game and disables the mod in all profiles
///
/// # Errors
///  - Profiles cannot be gotten
///  - Removing the files fails
pub fn uninstall_mod(name: &String) -> Result<(), ToryggError> {
    // TODO: check mod is installed

    for mut profile in Torygg::profiles()? {
        profile.deactivate_mod(name)?;
    }

    let mod_dir = config::mods_dir().join(name);
    fs::remove_dir_all(mod_dir).map_err(ToryggError::IOError)
}
