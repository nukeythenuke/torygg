use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use log::info;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use crate::config;
use crate::config::data_dir;
use crate::error::ToryggError;
use crate::games::SKYRIM_SPECIAL_EDITION;
use crate::profile::{Profile, profiles};
use crate::util::{find_case_insensitive_path, verify_directory};

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
#[derive(Serialize, Deserialize)]
pub struct ToryggState {
    //game: &'static SteamApp,
    #[serde(with = "serde_profile")]
    profile: Profile,
    deployed: Option<Vec<PathBuf>>
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
            profile: profiles().unwrap().first().unwrap().clone(),
            deployed: None
        };
        state.write().unwrap();
        state
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub fn profile_mut(&mut self) -> &mut Profile {
        &mut self.profile
    }

    pub fn set_profile(&mut self, name: &str) -> Result<(), ToryggError> {
        self.profile = Profile::from_str(name).map_err(|_| ToryggError::Other("failed to find profile".to_owned()))?;
        self.write()?;
        Ok(())
    }

    fn path() -> PathBuf {
        data_dir().join(".toryggstate.toml")
    }

    fn read() -> Result<ToryggState, ToryggError> {
        let s = fs::read_to_string(Self::path())?;
        toml::from_str(&s).map_err(|_| ToryggError::Other("Failed to parse state toml".to_owned()))
    }

    fn write(&self) -> Result<(), std::io::Error> {
        fs::write(Self::path(), toml::to_string(self).unwrap())
    }

    pub fn read_or_new() -> ToryggState {
        ToryggState::read().unwrap_or_else(|_| ToryggState::new())
    }

    pub fn deploy(&mut self) -> Result<(), ToryggError> {
        if self.deployed.is_some() {
            return Err(ToryggError::Other("Already Deployed".to_owned()))
        }

        let Some(mods) = self.profile.enabled_mods() else {
            return Ok(())
        };

        let data_path = SKYRIM_SPECIAL_EDITION.install_dir().unwrap().join("Data");
        let unmanaged_files = WalkDir::new(&data_path).min_depth(1).into_iter()
            .filter_map(|entry| Some(entry.ok()?.path().to_owned()))
            .collect::<Vec<_>>();

        let backup_dir = data_dir().join("Backup");
        verify_directory(&backup_dir)?;

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

                    fs::create_dir(&to_path)?;
                    result.push(to_relative_path);
                } else {
                    info!("{} -> {}", relative_path.display(), to_relative_path.display());

                    if to_path.exists() && unmanaged_files.contains(&to_path) {
                        let backup_path = backup_dir.join(&to_relative_path);
                        for dir in to_relative_path.parent().unwrap() {
                            verify_directory(&backup_dir.join(dir))?;
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
            self.deployed = Some(result);
            self.write()?;
        }

        Ok(())
    }

    pub fn undeploy(&mut self) -> Result<(), ToryggError> {
        if let Some(deployed) = &self.deployed {
            let data_path = SKYRIM_SPECIAL_EDITION.install_dir()?.join("Data");
            for relative_path in deployed.iter().rev() {
                let path = data_path.join(relative_path);
                if path.is_dir() {
                    fs::remove_dir(path)?;
                } else {
                    fs::remove_file(path)?;
                }
            }

            self.deployed = None;
            self.write().unwrap();

            let backup_dir = data_dir().join("Backup");
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
        }

        Ok(())
    }
}