use std::fs;
use std::path::{Path, PathBuf};
use log::info;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use crate::{config, modmanager};
use crate::config::data_dir;
use crate::error::ToryggError;
use crate::existing_directory::ExistingDirectory;
use crate::fomod::FomodCallback;
use crate::games::SKYRIM_SPECIAL_EDITION;
use crate::profile::Profile;
use crate::util::find_case_insensitive_path;

mod serde_profile {
    use std::fmt::Formatter;
    use std::str::FromStr;
    use serde::{de, Deserializer, Serializer};
    use serde::de::{Visitor};
    use crate::profile::{Profile};

    struct ProfileVisitor;

    impl<'de> Visitor<'de> for ProfileVisitor {
        type Value = Profile;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(formatter, "name of a profile")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: de::Error {
            Profile::from_str(v).map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
        }
    }

    pub fn serialize<S>(profile: &Profile, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer  {
        serializer.serialize_str(profile.name())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Profile, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_str(ProfileVisitor)
    }
}

/// Torygg's persistent state
#[derive(Debug, Serialize, Deserialize)]
pub struct ToryggState {
    //game: &'static SteamApp,
    #[serde(with = "serde_profile")]
    profile: Profile,
    deployed_files: Option<Vec<PathBuf>>
}

impl Default for ToryggState {
    fn default() -> Self {
        Self::read_or_new()
    }
}

impl ToryggState {
    // fn game(&self) -> &'static SteamApp {
    //     self.game
    // }

    #[must_use]
    pub fn new() -> ToryggState {
        let state = ToryggState {
            profile: Self::default_profile(),
            deployed_files: None,
        };
        state.write().unwrap();
        state
    }

    fn default_profile() -> Profile {
        Self::profiles().unwrap().first().unwrap().clone()
    }

    pub fn deployed(&self) -> bool {
        self.deployed_files.is_some()
    }

    pub fn mods() -> Result<Vec<String>, ToryggError> {
        modmanager::installed_mods()
    }

    pub fn install_mod(archive: &Path, name: &String, fomod_callback: FomodCallback) -> Result<(), ToryggError> {
        modmanager::install_mod(archive, name, fomod_callback)
    }

    pub fn uninstall_mod(name: &String) -> Result<(), ToryggError> {
        modmanager::uninstall_mod(name)
    }

    pub fn create_mod(mod_name: &String) -> Result<(), ToryggError> {
        modmanager::create_mod(mod_name)
    }

    #[must_use]
    pub fn active_mods(&self) -> Option<&Vec<String>> {
        self.profile.enabled_mods()
    }

    #[must_use]
    pub fn mod_active(&self, mod_name: &String) -> bool {
        self.profile().mod_enabled(mod_name)
    }

    pub fn activate_mod(&mut self, name: &String) -> Result<(), ToryggError> {
        if self.deployed() {
            return Err(ToryggError::IsDeployed)
        }

        self.profile.activate_mod(name)
    }

    pub fn deactivate_mod(&mut self, name: &String) -> Result<(), ToryggError> {
        if self.deployed() {
            return Err(ToryggError::IsDeployed)
        }

        self.profile.deactivate_mod(name)
    }

    pub fn profiles() -> Result<Vec<Profile>, ToryggError> {
        let profs = fs::read_dir(config::config_dir())?
            .filter_map(|e| Some(e.ok()?.path()))
            .filter_map(|e| ExistingDirectory::try_from(e).ok())
            .filter_map(|e| Profile::from_dir(&e).ok())
            .collect::<Vec<_>>();

        if profs.is_empty() {
            Profile::new("Default").unwrap();
            return Self::profiles()
        }

        Ok(profs)
    }

    #[must_use]
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    fn profile_mut(&mut self) -> &mut Profile {
        &mut self.profile
    }

    pub fn set_profile(&mut self, profile: Profile) -> Result<(), ToryggError> {
        if self.deployed() {
            return Err(ToryggError::IsDeployed)
        }

        self.profile = profile;
        self.write()?;
        Ok(())
    }

    pub fn create_profile(name: &str) -> Result<Profile, ToryggError> {
        Profile::new(name)
    }

    pub fn delete_profile(&mut self, profile: &Profile) -> Result<(), ToryggError> {
        if profile == self.profile() && self.deployed() {
            return Err(ToryggError::IsDeployed)
        }

        fs::remove_dir_all(profile.dir()?)?;

        if profile == self.profile() {
            self.profile = Self::default_profile();
        }

        Ok(())
    }

    fn path() -> PathBuf {
        data_dir().as_ref().join(".toryggstate.toml")
    }

    fn read() -> Result<ToryggState, ToryggError> {
        let s = fs::read_to_string(Self::path())?;
        toml::from_str::<ToryggState>(&s).map_err(|_| ToryggError::Other("Failed to parse state toml".to_owned()))
    }

    fn write(&self) -> Result<(), std::io::Error> {
        fs::write(Self::path(), toml::to_string(self).unwrap())
    }

    #[must_use]
    pub fn read_or_new() -> ToryggState {
        ToryggState::read().unwrap_or_else(|_| ToryggState::new())
    }

    pub fn deploy(&mut self) -> Result<(), ToryggError> {
        if self.deployed() {
            return Err(ToryggError::Other("Already Deployed".to_owned()))
        }

        // If there are no mods to deploy then we don't need to do anything
        let Some(mods) = self.profile.enabled_mods() else {
            return Ok(())
        };

        // Take note of pre-existing files
        let data_path = SKYRIM_SPECIAL_EDITION.install_dir().unwrap().join("Data");
        let unmanaged_files = WalkDir::new(&data_path).min_depth(1).into_iter()
            .filter_map(|entry| Some(entry.ok()?.path().to_owned()))
            .collect::<Vec<_>>();

        let backup_dir = data_dir().maybe_create_child_directory("Backup")?;

        let mut result  = Vec::new();
        for m in mods {
            let dir = config::mods_dir().existing_child_directory(m)
                .expect("mod directory does not exist");

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

                    fs::create_dir(&to_path)?;
                    result.push(to_relative_path);
                } else {
                    info!("{} -> {}", relative_path.display(), to_relative_path.display());

                    if to_path.exists() && unmanaged_files.contains(&to_path) {
                        let backup_path = backup_dir.as_ref().join(&to_relative_path);
                        for dir in to_relative_path.parent().unwrap() {
                            let _ = backup_dir.maybe_create_child_directory(dir)?;
                        }
                        fs::rename(&to_path, &backup_path)?;
                    }

                    fs::copy(path, &to_path)?;
                    if !result.contains(&to_relative_path) {
                        result.push(to_relative_path);
                    }
                }
            }
        }

        if !result.is_empty() {
            self.deployed_files = Some(result);
            self.write()?;
        }

        Ok(())
    }

    pub fn undeploy(&mut self) -> Result<(), ToryggError> {
        let Some(deployed) = &self.deployed_files else {
            return Err(ToryggError::IsNotDeployed)
        };

        // Remove mod files
        let data_path = SKYRIM_SPECIAL_EDITION.install_dir()?.join("Data");
        for relative_path in deployed.iter().rev() {
            let path = data_path.join(relative_path);
            if path.is_dir() {
                fs::remove_dir(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }

        self.deployed_files = None;
        self.write().unwrap();

        // Restore any backed up files
        let backup_dir = data_dir().maybe_create_child_directory("Backup")?;
        for entry in WalkDir::new(&backup_dir).min_depth(1).contents_first(true) {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative_path = path.strip_prefix(&backup_dir).unwrap();
            let to_path = data_path.join(relative_path);

            if path.is_file() {
                info!("{}", relative_path.display());
                fs::rename(path, to_path).unwrap();
            } else {
                fs::remove_dir(path).unwrap();
            }
        }

        Ok(())
    }
}