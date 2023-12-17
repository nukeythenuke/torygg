use std::fs;
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::{config, modmanager};
use crate::config::data_dir;
use crate::games::SKYRIM_SPECIAL_EDITION;
use crate::util::{find_case_insensitive_path, verify_directory};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Profile {
    name: String,
    mods: Option<Vec<String>>,
}

impl std::str::FromStr for Profile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for profile in profiles().map_err(|e| anyhow!(e))? {
            if profile.name() == s {
                return Ok(profile)
            }
        }

        Err(anyhow!("Profile not found"))
    }
}

impl Profile {
    pub fn new(profile_name: &str) -> Result<Profile, ToryggError> {
        let path = config::config_dir().join(profile_name);
        if path.exists() {
            return Err(ToryggError::ProfileAlreadyExists)
        }

        verify_directory(&path)?;
        let profile = Profile { name: profile_name.to_string(), mods: None };
        profile.write()?;
        Ok(profile)
    }

    fn write(&self) -> Result<(), ToryggError> {
        let string = match toml::to_string(self) {
            Ok(s) => s,
            Err(e) => return Err(ToryggError::Other(e.to_string()))
        };

        match fs::write(self.dir()?.join("profile.toml"), string) {
            Ok(()) => Ok(()),
            Err(e) => Err(ToryggError::IOError(e))
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn from_dir(profile_dir: &Path) -> Result<Profile, ToryggError> {
        let Ok(profile_string) = fs::read_to_string(profile_dir.join("profile.toml")) else {
            return Err(ToryggError::Other("failed to read profile.toml".to_owned()));
        };

        match toml::from_str::<Profile>(&profile_string) {
            Ok(profile) => Ok(profile),
            Err(e) => Err(ToryggError::Other(e.to_string()))
        }
    }

    fn set_mod_enabled(&mut self, mod_name: &String, enabled: bool) -> Result<(), ToryggError> {
        if !modmanager::mod_installed(mod_name)? {
            return Err(ToryggError::Other(String::from("Mod not installed")));
        }

        if self.mods.is_none() {
            self.mods = Some(Vec::new());
        }

        // Should be safe as we have checked if self.mods is None and assigned it if not
        let mods = self.mods.as_mut().unwrap();

        if enabled {
            if !mods.contains(mod_name) {
                mods.push(mod_name.to_owned());
                self.write()?;
            }
        } else if mods.contains(mod_name) {
            *mods = mods.clone().into_iter().filter(|name| name != mod_name).collect();
            if mods.is_empty() {
                self.mods = None;
            }

            self.write()?;
        }

        Ok(())
    }

    pub fn enable_mod(&mut self, mod_name: &String) -> Result<(), ToryggError> {
        self.set_mod_enabled(mod_name, true)
    }

    pub fn disable_mod(&mut self, mod_name: &String) -> Result<(), ToryggError> {
        self.set_mod_enabled(mod_name, false)
    }

    #[must_use]
    pub fn mod_enabled(&self, mod_name: &String) -> bool {
        match &self.mods {
            Some(mods) => mods.contains(mod_name),
            None => false
        }
    }

    #[must_use]
    pub fn enabled_mods(&self) -> Option<&Vec<String>> {
        self.mods.as_ref()
    }

    pub fn dir(&self) -> Result<PathBuf, ToryggError> {
        let dir = config::config_dir().join(&self.name);
        verify_directory(&dir)?;
        Ok(dir)
    }

    pub fn mods_dir(&self) -> Result<&PathBuf, ToryggError> {
        Ok(config::mods_dir())
    }

    pub fn deploy(&self) -> Result<Option<Vec<PathBuf>>, ToryggError> {
        let Some(mods) = self.enabled_mods() else {
            return Ok(None)
        };

        let data_path = SKYRIM_SPECIAL_EDITION.install_dir().unwrap().join("Data");
        let unmanaged_files = WalkDir::new(&data_path).min_depth(1).into_iter()
            .filter_map(|entry| Some(entry.ok()?.path().to_owned()))
            .collect::<Vec<_>>();

        let backup_dir = data_dir().join("Backup");
        verify_directory(&backup_dir).unwrap();

        let mut result  = Vec::new();
        for m in mods {
            let dir = config::mods_dir().join(m);
            for entry in WalkDir::new(&dir).min_depth(1) {
                let entry = entry.unwrap();
                let path = entry.path();


                let relative_path = path.strip_prefix(&dir).unwrap();
                let to_relative_path = find_case_insensitive_path(&data_path, relative_path);
                let to_path = data_path.join(&to_relative_path);

                if path.is_dir() {
                    if to_path.is_dir() {
                        continue;
                    }

                    fs::create_dir(&to_path).unwrap();
                    result.push(to_relative_path);
                } else {
                    println!("{} -> {}", relative_path.display(), to_relative_path.display());

                    if to_path.exists() && unmanaged_files.contains(&to_path) {
                        let backup_path = backup_dir.join(&to_relative_path);
                        for dir in to_relative_path.parent().unwrap() {
                            verify_directory(&backup_dir.join(dir)).unwrap();
                        }
                        fs::rename(&to_path, &backup_path).unwrap();
                    }

                    fs::copy(path, &to_path).unwrap();
                    if !result.contains(&to_relative_path) {
                        result.push(to_relative_path);
                    }
                }
            }
        }

        Ok(Some(result))
    }
}

pub fn profiles() -> Result<Vec<Profile>, ToryggError> {
    let profs = fs::read_dir(config::config_dir())?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter_map(|e| {
            if e.is_dir() {
                Profile::from_dir(&e).ok()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if profs.is_empty() {
        Profile::new("Default").unwrap();
        return profiles()
    }

    Ok(profs)
}