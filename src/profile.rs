use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use walkdir::WalkDir;
use crate::error::ToryggError;
use crate::config;
use crate::util::verify_directory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    name: String,
    is_enabled: bool,
    plugins: HashMap<String, bool>
}

impl Mod {
    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_is_enabled(&self) -> &bool {
        &self.is_enabled
    }

    pub fn get_plugins(&self) -> &HashMap<String, bool> {
        &self.plugins
    }
}

#[derive(Debug, Clone)]
pub struct Profile {
    name: String,
    // Mod name, enabled
    mods: Option<Vec<Mod>>
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
    pub fn new(profile_name: &str) -> Result<Profile, ToryggError> {
        let path = config::get_profiles_dir().join(profile_name);
        if path.exists() {
            Err(ToryggError::ProfileAlreadyExists)
        } else {
            verify_directory(&path)?;
            Ok(Profile { name: profile_name.to_string(), mods: None })
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    fn read_mod_meta(path: &Path) -> Mod {
        let s = std::fs::read_to_string(path).expect("failed to read mod meta");
        toml::from_str(&s).expect("failed to deserialize mod meta")
    }

    fn write_mod_meta(path: &Path, m: &Mod) {
        let s = toml::to_string(&m).expect("failed to serialize mod meta");
        std::fs::write(path, s).expect("failed to write mod meta")
    }

    pub fn from_dir(profile_dir: PathBuf) -> Result<Profile, ToryggError> {
        let profile_name = profile_dir.file_name().unwrap().to_string_lossy().to_string();
        let mut profile = Profile { name: profile_name, mods: None };

        let dir_contents: Vec<PathBuf> = fs::read_dir(profile.get_mods_dir()?)?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect();

        let files: Vec<&PathBuf> = dir_contents.iter().filter(|path| path.is_file()).collect();
        let dirs = dir_contents.iter().filter(|path| path.is_dir());

        let mut mods = Vec::new();
        for dir in dirs {
            let mod_name = dir.file_stem().unwrap().to_string_lossy().to_string();
            let meta_name = mod_name.clone() + ".meta.toml";
            let meta_path = dir.join(meta_name);

            let m = if meta_path.exists() {
                Self::read_mod_meta(&meta_path)
            } else {
                // TODO: Find plugin files
                let m = Mod {
                    name: mod_name.clone(),
                    is_enabled: false,
                    plugins: HashMap::new()
                };

                Self::write_mod_meta(&meta_path, &m);
                m
            };

            mods.push(m);
        }

        if mods.is_empty() {
            profile.mods = Some(mods);
        }

        Ok(profile)
    }

    pub fn create_mod(&self, mod_name: &str) -> Result<(), ToryggError> {
        if !self.is_mod_installed(mod_name) {
            verify_directory(&self.get_mods_dir().unwrap().join(mod_name))
        } else {
            Err(ToryggError::ModAlreadyExists)
        }
    }

    pub fn install_mod(&self, archive: &Path, name: &str) -> Result<(), &'static str>{
        if !archive.exists() {
            Err("Archive does not exist!")
        } else if self.is_mod_installed(name) {
            Err("Mod already exists!")
        } else {
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
    }

    pub fn uninstall_mod(&self, name: &str) -> Result<(), &'static str> {
        todo!()
    }

    fn set_mod_enabled(&mut self, mod_name: &str, enabled: bool) -> Result<(), &'static str>{
        {
            let mut m = self.get_mod_mut(mod_name).ok_or("Mod not installed")?;
            m.is_enabled = enabled;
        }

        let meta_path = self.get_mods_dir().unwrap().join(mod_name).join(mod_name.to_owned() + ".meta.toml");
        Self::write_mod_meta(&meta_path, self.get_mod(mod_name).unwrap());

        Ok(())
    }

    pub fn enable_mod(&mut self, mod_name: &str) -> Result<(), &'static str> {
        self.set_mod_enabled(mod_name, true)
    }

    pub fn disable_mod(&mut self, mod_name: &str) -> Result<(), &'static str> {
        self.set_mod_enabled(mod_name, false)
    }

    fn is_mod_installed(&self, mod_name: &str) -> bool {
        self.get_mod(mod_name).is_some()
    }

    pub fn is_mod_enabled(&self, mod_name: &str) -> Result<&bool, &'static str> {
        let res = self.get_mod(mod_name).ok_or("mod not installed")?.get_is_enabled();
        Ok(res)
    }

    pub fn get_mod(&self, mod_name: &str) -> Option<&Mod> {
        let Some(mods) = &self.mods else {
            return None;
        };

        for m in mods {
            if m.name == mod_name {
                return Some(m)
            }
        }

        None
    }

    pub fn get_mod_mut(&mut self, mod_name: &str) -> Option<&mut Mod> {
        let Some(mods) = &mut self.mods else {
            return None;
        };

        for m in mods {
            if m.name == mod_name {
                return Some(m)
            }
        }

        None
    }

    pub fn get_mods(&self) -> &Option<Vec<Mod>> {
        &self.mods
    }

    pub fn get_enabled_mods(&self) -> Vec<&String> {
        let Some(mods) = &self.mods else {
            return Vec::new()
        };

        mods.iter().filter_map(|m| match m.get_is_enabled() {
            true => Some(m.get_name()),
            false => None
        }).collect()
    }

    pub fn get_dir(&self) -> Result<PathBuf, ToryggError> {
        let dir = config::get_profiles_dir().join(&self.name);
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
    Ok(fs::read_dir(config::get_profiles_dir())?
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