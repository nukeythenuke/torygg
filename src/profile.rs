use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::{config, games};
use crate::util::verify_directory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    game: String,
    name: String,
    mods: Option<Vec<String>>,
    plugins: Option<Vec<String>>
}

impl std::str::FromStr for Profile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for profile in get_profiles().map_err(|e| anyhow!(e))? {
            if profile.get_name() == s {
                return Ok(profile)
            }
        }

        Err(anyhow!("Profile not found"))
    }
}

impl Profile {
    pub fn new(profile_name: &str, game: impl games::Game) -> Result<Profile, ToryggError> {
        let path = config::get_config_dir().join(profile_name);
        if path.exists() {
            Err(ToryggError::ProfileAlreadyExists)
        } else {
            verify_directory(&path)?;
            Ok(Profile { game: game.get_name().to_owned(), name: profile_name.to_string(), mods: None, plugins: None })
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn from_dir(profile_dir: PathBuf) -> Result<Profile, ToryggError> {
        let Ok(profile_string) = std::fs::read_to_string(profile_dir.join("profile.toml")) else {
            return Err(ToryggError::Other("failed to read profile.toml".to_owned()));
        };

        match toml::from_str::<Profile>(&profile_string) {
            Ok(profile) => Ok(profile),
            Err(e) => Err(ToryggError::Other(e.to_string()))
        }
    }

    pub fn create_mod(&self, mod_name: &str) -> Result<(), ToryggError> {
        // TODO: check mod name is not already used (installed already)
        // Err(ToryggError::ModAlreadyExists)

        verify_directory(&self.get_mods_dir().unwrap().join(mod_name))
    }

    pub fn install_mod(&self, archive: &Path, name: &str) -> Result<(), &'static str>{
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
        let install_path = self.get_mods_dir().unwrap().join(name);
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

    pub fn uninstall_mod(&self, name: &str) -> Result<(), &'static str> {
        todo!()
    }

    fn set_mod_enabled(&mut self, mod_name: &str, enabled: bool) {
        // TODO: Check if mod is installed

        if self.mods.is_none() {
            self.mods = Some(Vec::new());
        }

        // Should be safe as we have checked if self.mods is None and assigned it if not
        let mods = self.mods.as_mut().unwrap();

        if enabled {
            if !mods.contains(&mod_name.to_owned()) {
                mods.push(mod_name.to_owned());
            }
        } else if mods.contains(&mod_name.to_owned()) {
            *mods = mods.clone().into_iter().filter(|name| name != mod_name).collect();
        }
    }

    pub fn enable_mod(&mut self, mod_name: &str) {
        self.set_mod_enabled(mod_name, true)
    }

    pub fn disable_mod(&mut self, mod_name: &str) {
        self.set_mod_enabled(mod_name, false)
    }

    pub fn is_mod_enabled(&self, mod_name: &String) -> bool {
        match &self.mods {
            Some(mods) => mods.contains(mod_name),
            None => false
        }
    }

    pub fn get_enabled_mods(&self) -> &Option<Vec<String>> {
        &self.mods
    }

    pub fn get_dir(&self) -> Result<PathBuf, ToryggError> {
        let dir = config::get_config_dir().join(&self.name);
        verify_directory(&dir)?;
        Ok(dir)
    }

    fn get_appdata_dir(&self) -> Result<PathBuf, ToryggError> {
        let dir = self.get_dir()?.join("AppData");
        verify_directory(&dir)?;
        Ok(dir)
    }

    pub fn get_mods_dir(&self) -> Result<PathBuf, ToryggError> {
        let dir = self.get_dir()?.join("Mods");
        verify_directory(&dir)?;
        Ok(dir)
    }

    pub fn get_overwrite_dir(&self) -> Result<PathBuf, ToryggError> {
        let dir = self.get_dir()?.join("Overwrite");
        verify_directory(&dir)?;
        Ok(dir)
    }
}

pub fn get_profiles() -> Result<Vec<Profile>, ToryggError> {
    Ok(fs::read_dir(config::get_config_dir())?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter_map(|e| {
            if e.is_dir() {
                Profile::from_dir(e).ok()
            } else {
                None
            }
        })
        .collect())
}