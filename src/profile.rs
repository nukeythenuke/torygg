use std::fs;
use std::path::PathBuf;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use crate::error::ToryggError;
use crate::config;
use crate::games::SteamApp;
use crate::util::verify_directory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    game: SteamApp,
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
    pub fn new(profile_name: &str, game: SteamApp) -> Result<Profile, ToryggError> {
        let path = config::get_config_dir().join(profile_name);
        if path.exists() {
            return Err(ToryggError::ProfileAlreadyExists)
        }

        verify_directory(&path)?;
        let profile = Profile { game, name: profile_name.to_string(), mods: None, plugins: None };
        profile.write()?;
        Ok(profile)
    }

    fn write(&self) -> Result<(), ToryggError> {
        let string = match toml::to_string(self) {
            Ok(s) => s,
            Err(e) => return Err(ToryggError::Other(e.to_string()))
        };

        match std::fs::write(self.get_dir()?.join("profile.toml"), string) {
            Ok(_) => Ok(()),
            Err(e) => Err(ToryggError::IOError(e))
        }
    }

    pub  fn get_game(&self) -> &SteamApp {
        &self.game
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