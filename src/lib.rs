pub mod applauncher;
pub mod config;
pub mod games;

use std::collections::HashMap;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::anyhow;
use log::error;
use tempfile::TempDir;
use walkdir::WalkDir;

use crate::games::Game;
use crate::util::verify_directory;

pub mod wine {
    use std::collections::HashMap;
    use std::path::PathBuf;

    pub struct Prefix {
        pub wine_exec: PathBuf,
        pub pfx: PathBuf,
        pub env: HashMap<String, String>,
    }

    impl Prefix {
        pub fn new(pfx: PathBuf) -> Prefix {
            Prefix {
                // TODO: Find the correct wine executable
                wine_exec: Default::default(),
                pfx,
                // TODO: Find the correct environment variables
                env: Default::default(),
            }
        }
    }
}

pub mod util {
    use std::{fs, fs::File, iter::FromIterator, path::PathBuf};
    use std::path::Path;
    use crate::games;

    pub fn get_libraryfolders_vdf() -> PathBuf {
        PathBuf::from(std::env::var("HOME").unwrap()).join(".steam/root/config/libraryfolders.vdf")
    }

    pub fn get_steam_library(app: &games::SteamApp) -> Option<PathBuf> {
        let vdf = get_libraryfolders_vdf();
        let mut file = File::open(vdf).ok()?;
        let kvs = torygg_vdf::parse(&mut file).ok()?;

        for kv in &kvs {
            let components = kv.0.iter().collect::<Vec<_>>();
            // Key we want:                    🠗
            // libraryfolders/<lib_id>/apps/<appid>
            if let Some(component) = components.get(3) {
                if *component == app.appid.to_string().as_str() {
                    // libraryfolders/<lib_id>/path
                    let path = PathBuf::from_iter(kv.0.iter().take(2)).join("path");

                    return Some(kvs[&path].clone().into());
                }
            }
        }

        None
    }

    pub fn verify_directory(path: &Path) -> Result<(), &'static str> {
        if path.exists() {
            path.is_dir()
                .then(|| ())
                .ok_or("path exists but is not a directory")
        } else {
            fs::create_dir(path).map_err(|_| "Couldn't create directory")
        }
    }
}

pub fn get_profiles() -> Result<Vec<Profile>, &'static str> {
    Ok(fs::read_dir(config::get_profiles_dir()?)
        .map_err(|_| "Could not read profiles dir")?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter_map(|e| {
            e.is_dir()
                .then(|| ())
                .and_then(|_| Profile::from_dir(e).ok())
        })
        .collect())
}

#[derive(Clone)]
pub struct Profile {
    name: String,
    // Mod name, enabled
    mods: HashMap<String, bool>
}

impl Deref for Profile {
    type Target = HashMap<String, bool>;

    fn deref(&self) -> &Self::Target {
        &self.mods
    }
}

impl DerefMut for Profile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mods
    }
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
    pub fn new(profile_name: &str) -> Result<Profile, &'static str> {
        let path = config::get_profiles_dir()?.join(profile_name);
        if path.exists() {
            Err("Profile already exists!")
        } else {
            verify_directory(&path)?;
            Ok(Profile { name: profile_name.to_string(), mods: HashMap::new() })
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
                               // Enabled, plugins
    fn read_meta(path: &Path) -> (bool, Vec<String>) {
        todo!()
    }

    fn write_meta(mod_name: &str, contents: (bool, Vec<String>)) {
        todo!()
    }

    fn from_dir(mod_dir: PathBuf) -> Result<Profile, &'static str> {
        let dir_contents: Vec<PathBuf> = fs::read_dir(&mod_dir)
            .map_err(|_| "Could not read mods dir")?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect();

        let files: Vec<&PathBuf> = dir_contents.iter().filter(|path| path.is_file()).collect();
        let dirs = dir_contents.iter().filter(|path| path.is_dir());

        let mut mod_map = HashMap::new();
        for dir in dirs {
            let mod_name = dir.file_stem().unwrap();

            // TODO: Better than this
            let mut found_meta = false;
            let mut is_enabled = false;
            for file in files.iter() {
                if file.file_stem().unwrap() == mod_name {
                    found_meta = true;
                    is_enabled = Self::read_meta(file.to_owned()).0;
                    break;
                }
            }

            if !found_meta {
                todo!("Create a new meta file")
            }

            mod_map.insert(mod_name.to_string_lossy().to_string(), is_enabled);
        }

        // TODO: Clean up meta files that do not have an associated mod directory

        // TODO: Fetch a specific profile
        Ok(Profile { name: "default".to_owned(), mods: mod_map })
    }

    pub fn create_mod(&self, mod_name: &str) -> Result<(), &'static str> {
        if !self.is_mod_installed(mod_name) {
            verify_directory(&self.get_mods_dir().unwrap().join(mod_name))
        } else {
            Err("Mod with same name already exists!")
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
            command.arg(&archive);

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
        let res = if self.deref().contains_key(mod_name) {
            self.deref_mut().insert(mod_name.to_owned(), enabled);
            Ok(())
        } else {
            Err("Mod not installed")
        };

        if res.is_ok() {
            // TODO: Find plugins
            Self::write_meta(mod_name, (enabled, Vec::new()))
        }

        res
    }

    pub fn enable_mod(&mut self, mod_name: &str) -> Result<(), &'static str> {
        self.set_mod_enabled(mod_name, true)
    }

    pub fn disable_mod(&mut self, mod_name: &str) -> Result<(), &'static str> {
        self.set_mod_enabled(mod_name, false)
    }

    fn is_mod_installed(&self, mod_name: &str) -> bool {
        self.mods.contains_key(mod_name)
    }

    pub fn is_mod_enabled(&self, mod_name: &str) -> Result<&bool, &'static str> {
        self.mods.get(mod_name).ok_or("Mod not installed")
    }

    pub fn get_mods(&self) -> &HashMap<String, bool> {
        &self.mods
    }

    fn get_enabled_mods(&self) -> Vec<&String> {
        self.mods.iter().filter_map(|(name, enabled)| match enabled {
            true => Some(name),
            false => None
        }).collect()
    }

    pub fn get_dir(&self) -> Result<PathBuf, &'static str> {
        let dir = config::get_profiles_dir()?.join(&self.name);
        verify_directory(&dir)?;
        Ok(dir)
    }

    fn get_appdata_dir(&self) -> Result<PathBuf, &'static str> {
        let dir = self.get_dir()?.join("AppData");
        verify_directory(&dir)?;
        Ok(dir)
    }

    fn get_mods_dir(&self) -> Result<PathBuf, &'static str> {
        let dir = self.get_dir()?.join("Mods");
        verify_directory(&dir)?;
        Ok(dir)
    }
}