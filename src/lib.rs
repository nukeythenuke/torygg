use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{error, info};
use tempfile::TempDir;
use walkdir::WalkDir;
use crate::games::Game;

static APP_NAME: &str = "torygg";

static MODS_SUBDIR: &str = "Mods";
static OVERWRITE_SUBDIR: &str = "Overwrite";
static PROFILES_SUBDIR: &str = "Profiles";

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

pub mod games {
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use log::info;
    use crate::util;
    use crate::wine::Prefix;

    pub trait Game {
        fn get_install_dir(&self) -> Option<PathBuf>;
        fn get_executable(&self) -> Option<PathBuf>;
        fn get_wine_pfx(&self) -> Option<Prefix>;
        fn get_name(&self) -> &'static str;
        fn run(&self) -> Result<(), &'static str>;
    }

    /// appid: Steam app id
    /// install_dir: Directory inside "$LIBRARY/steamapps/common" that the app is installed into
    /// executable: game executable
    /// mod_loader_executable: eg. skse64_loader.exe
    #[derive(Debug)]
    pub struct SteamApp {
        pub appid: usize,
        pub install_dir: &'static str,
        pub executable: &'static str,
        pub mod_loader_executable: Option<&'static str>,
    }

    impl Game for SteamApp {
        fn get_install_dir(&self) -> Option<PathBuf> {
            let path = util::get_steam_library(self)?
                .join("steamapps/common")
                .join(self.install_dir);

            if path.exists() {
                Some(path)
            } else {
                None
            }
        }
        fn get_executable(&self) -> Option<PathBuf> {
            let install_dir = self.get_install_dir()?;
            if let Some(mle) = self.mod_loader_executable {
                let mle_path = install_dir.join(mle);
                if mle_path.exists() {
                    return Some(mle_path);
                }
            }

            Some(install_dir.join(self.executable))
        }

        fn get_wine_pfx(&self) -> Option<Prefix> {
            let path = util::get_steam_library(self)?
                .join("steamapps/compatdata")
                .join(self.appid.to_string())
                .join("pfx");

            Some(Prefix::new(path))
        }

        fn get_name(&self) -> &'static str {
            self.install_dir
        }

        fn run(&self) -> Result<(), &'static str> {
            let install_dir = self.get_install_dir().unwrap();
            let executable = self.get_executable().unwrap();

            info!("Starting protontricks");
            let mut cmd = Command::new("protontricks");
            cmd.arg(self.appid.to_string());
            cmd.arg("shell");
            cmd.stdin(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(_) => return Err("Failed to spawn child"),
            };

            if child
                .stdin
                .take()
                .unwrap()
                .write_all(
                    format!(
                        "cd \"{}\" && wine \"{}\"\n",
                        install_dir.display(),
                        executable.display()
                    )
                        .as_bytes(),
                )
                .is_err()
            {
                return Err("failed to write to child");
            }

            let status = match child.wait() {
                Ok(status) => status,
                Err(_) => return Err("Child failed"),
            };

            if !status.success() {
                return Err("Child failed");
            }

            Ok(())
        }
    }

    pub const SKYRIM: SteamApp = SteamApp {
        appid: 72850,
        install_dir: "Skyrim",
        executable: "Skyrim.exe",
        mod_loader_executable: None,
    };
    pub const SKYRIM_SPECIAL_EDITION: SteamApp = SteamApp {
        appid: 489830,
        install_dir: "Skyrim Special Edition",
        executable: "SkyrimSE.exe",
        mod_loader_executable: Some("skse64_loader.exe"),
    };

    impl std::str::FromStr for &SteamApp {
        type Err = anyhow::Error;

        fn from_str(s: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
            Ok(match s {
                "skyrim" => &SKYRIM,
                "skyrimse" => &SKYRIM_SPECIAL_EDITION,
                _ => anyhow::bail!("Unknown game \"{s}\"")
            })
        }
    }
}

pub mod util {
    use std::{fs::File, iter::FromIterator, path::PathBuf};
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

    pub fn get_install_dir(app: &games::SteamApp) -> Option<PathBuf> {
        let path = get_steam_library(app)?
            .join("steamapps/common")
            .join(app.install_dir);

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_wine_prefix(app: &games::SteamApp) -> Option<PathBuf> {
        let path = get_steam_library(app)?
            .join("steamapps/compatdata")
            .join(app.appid.to_string())
            .join("pfx");

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
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

pub fn install_mod_from_archive(archive_path: &Path, mod_name: &str) -> Result<(), &'static str> {
    if !archive_path.exists() {
        Err("Archive does not exist!")
    } else if is_mod_installed(mod_name)? {
        Err("Mod already exists!")
    } else {
        let archive_extract_dir = TempDir::new().unwrap();
        let archive_extract_path = archive_extract_dir.into_path();

        // Use p7zip to extract the archive to a temporary directory
        let mut command = Command::new("7z");
        command.arg("x");
        command.arg(format!("-o{}", archive_extract_path.display()));
        command.arg(&archive_path);

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
        let install_path = get_mods_dir().unwrap().join(mod_name);
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

pub fn create_mod(mod_name: &str) -> Result<(), &'static str> {
    if !is_mod_installed(mod_name)? {
        verify_directory(&get_mods_dir().unwrap().join(mod_name))
    } else {
        Err("Mod with same name already exists!")
    }
}

pub fn uninstall_mod(mod_name: &str) -> Result<(), &'static str> {
    if is_mod_installed(mod_name)? {
        for p in get_profiles()? {
            deactivate_mod(&p, mod_name).ok();
        }

        fs::remove_file(get_mod_dir(mod_name).unwrap()).map_err(|_| "Failed to remove file")
    } else {
        Err("Mod not installed")
    }
}

fn get_mod_dir(mod_name: &str) -> Result<PathBuf, &'static str> {
    let dir = get_mods_dir()?.join(mod_name);
    dir.exists().then(|| dir).ok_or("mod dir does not exist")
}

pub fn get_installed_mods() -> Result<Vec<String>, &'static str> {
    Ok(fs::read_dir(get_mods_dir()?)
        .map_err(|_| "Could not read mods dir")?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter_map(|e| {
            (e.is_dir())
                .then(|| ())
                .and_then(|_| Some(e.file_stem()?.to_str()?.to_owned()))
        })
        .collect())
}

fn is_mod_installed(mod_name: &str) -> Result<bool, &'static str> {
    Ok(get_installed_mods()?.contains(&mod_name.to_owned()))
}

pub fn get_profiles() -> Result<Vec<String>, &'static str> {
    Ok(fs::read_dir(get_profiles_dir()?)
        .map_err(|_| "Could not read profiles dir")?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter_map(|e| {
            e.is_dir()
                .then(|| ())
                .and_then(|_| Some(e.file_stem()?.to_str()?.to_owned()))
        })
        .collect())
}

pub fn create_profile(profile_name: &str) -> Result<(), &'static str> {
    let path = get_profiles_dir()?.join(profile_name);
    if path.exists() {
        Err("Profile already exists!")
    } else {
        verify_directory(&path)?;
        Ok(())
    }
}

pub fn activate_mod(profile_name: &str, mod_name: &str) -> Result<(), &'static str> {
    if !is_mod_installed(mod_name)? {
        Err("Mod not installed")
    } else if is_mod_active(profile_name, mod_name)? {
        Err("Mod already active")
    } else {
        // Discover plugins
        let mod_dir = get_mods_dir()?.join(mod_name);
        let plugins = fs::read_dir(mod_dir)
            .map_err(|_| "Failed to read mod dir")?
            .filter_map(|e| Some(e.ok()?.path()))
            .filter(|e| e.extension().is_some())
            .filter(|e| e.extension().unwrap().to_str().is_some())
            .filter(|e| {
                matches!(
                    e.extension().unwrap().to_str().unwrap(),
                    "esp" | "esm" | "esl"
                )
            })
            .filter_map(|e| {
                (!e.is_dir())
                    .then(|| ())
                    .and_then(|_| Some(e.file_name()?.to_str()?.to_owned()))
            })
            .collect::<Vec<String>>();

        fs::write(
            get_profile_mods_dir(profile_name)?.join(mod_name),
            plugins.join("\n"),
        )
            .map_err(|_| "Failed to write to profile's mods dir")?;

        let plugins_path = get_profile_appdata_dir(profile_name)?.join("Plugins.txt");
        let plugins_string = if plugins_path.exists() {
            fs::read_to_string(&plugins_path).map_err(|_| "Could not read Plugins.txt")?
        } else {
            "".to_owned()
        };

        let mut plugins_vec = if plugins_string.is_empty() {
            Vec::new()
        } else {
            plugins_string.split('\n').map(|s| s.to_owned()).collect()
        };

        for plugin in plugins {
            plugins_vec.push(format!("*{}", plugin));
        }

        fs::write(plugins_path, plugins_vec.join("\n")).map_err(|e| {
            error!("{}", e.to_string());
            "Failed to write Plugins.txt"
        })
    }
}

pub fn deactivate_mod(profile_name: &str, mod_name: &str) -> Result<(), &'static str> {
    if !is_mod_installed(mod_name)? {
        Err("Mod not installed")
    } else if !is_mod_active(profile_name, mod_name)? {
        Err("Mod is not active")
    } else {
        let plugins_path = get_profile_appdata_dir(profile_name)?.join("Plugins.txt");
        let plugins_string = if plugins_path.exists() {
            fs::read_to_string(&plugins_path).map_err(|_| "Could not read Plugins.txt")?
        } else {
            "".to_owned()
        };

        let mut plugins_vec = if plugins_string.is_empty() {
            Vec::new()
        } else {
            plugins_string.split('\n').map(|s| s.to_owned()).collect()
        };

        let mod_plugins_string =
            fs::read_to_string(get_profile_mods_dir(profile_name)?.join(mod_name))
                .map_err(|_| "Failed to read mod plugins")?;

        let mod_plugins_vec = if plugins_string.is_empty() {
            Vec::new()
        } else {
            mod_plugins_string
                .split('\n')
                .map(|s| s.to_owned())
                .collect()
        };

        let mod_plugins_vec_active = mod_plugins_vec
            .iter()
            .map(|s| format!("*{}", s))
            .collect::<Vec<String>>();

        for i in 0..plugins_vec.len() {
            if mod_plugins_vec.contains(&plugins_vec[i]) {
                plugins_vec.remove(i);
            }
            if mod_plugins_vec_active.contains(&plugins_vec[i]) {
                plugins_vec.remove(i);
            }
        }

        fs::remove_file(get_profile_mods_dir(profile_name)?.join(mod_name))
            .map_err(|_| "Failed to remove mod file")?;
        fs::write(plugins_path, plugins_vec.join("\n")).map_err(|e| {
            error!("{}", e.to_string());
            "Failed to write Plugins.txt"
        })
    }
}

fn get_active_mods(profile_name: &str) -> Result<Vec<String>, &'static str> {
    Ok(fs::read_dir(get_profile_mods_dir(profile_name)?)
        .map_err(|_| "Could not read dir")?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter_map(|e| {
            (!e.is_dir())
                .then(|| ())
                .and_then(|_| Some(e.file_stem()?.to_str()?.to_owned()))
        })
        .collect::<Vec<String>>())
}

pub fn is_mod_active(profile_name: &str, mod_name: &str) -> Result<bool, &'static str> {
    Ok(get_active_mods(profile_name)?.contains(&mod_name.to_owned()))
}

fn get_wine_user_dir() -> Result<PathBuf, &'static str> {
    match std::env::var_os("TORYGG_USER_DIRECTORY") {
        Some(str) => {
            let path = PathBuf::from(str);
            if path.exists() {
                Ok(path)
            } else {
                Err("specified path does not exist")
            }
        }
        None => {
            let err = Err("wine user dir not found");
            let path = (games::SKYRIM_SPECIAL_EDITION).get_wine_pfx()
                .ok_or("skyrim install dir not found")?
                .pfx;
            let mut path = path.clone();
            path.push("drive_c/users");
            let steamuser = path.join("steamuser");
            if steamuser.exists() {
                Ok(steamuser)
            } else if let Some(current_user) =
                std::env::vars().collect::<HashMap<_, _>>().get("USER")
            {
                let user_dir = path.join(current_user);
                if user_dir.exists() {
                    Ok(user_dir)
                } else {
                    err
                }
            } else {
                err
            }
        }
    }
}

fn get_config_dir(game: &dyn Game) -> Result<PathBuf, &'static str> {
    Ok(get_wine_user_dir()?.join(String::from("My Documents/My Games/") + game.get_name()))
}

// Folder where profile Plugins.txt is kept
fn get_appdata_dir(game: &dyn Game) -> Result<PathBuf, &'static str> {
    Ok(get_wine_user_dir()?
        .join(String::from("Local Settings/Application Data/") + game.get_name()))
}

pub fn get_data_dir() -> Result<PathBuf, &'static str> {
    let dir = dirs::data_dir()
        .ok_or("Could not find torygg's data dir")?
        .join(APP_NAME);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_mods_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(MODS_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_overwrite_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(OVERWRITE_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_profiles_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(PROFILES_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_profile_dir(profile_name: &str) -> Result<PathBuf, &'static str> {
    let dir = get_profiles_dir()?.join(profile_name);
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_profile_mods_dir(profile_name: &str) -> Result<PathBuf, &'static str> {
    let dir = get_profile_dir(profile_name)?.join("Mods");
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_profile_appdata_dir(profile_name: &str) -> Result<PathBuf, &'static str> {
    let dir = get_profile_dir(profile_name)?.join("AppData");
    verify_directory(&dir)?;
    Ok(dir)
}

pub struct AppLauncher<'a> {
    app: &'static dyn Game,
    profile: &'a str,
    mounted_paths: Vec<PathBuf>,
}

impl<'a> AppLauncher<'a> {
    pub fn new(app: &'static dyn Game, profile: &'a str) -> Self {
        AppLauncher {
            app,
            profile,
            mounted_paths: Vec::new(),
        }
    }

    fn mount_path(
        &mut self,
        path: &Path,
        lower_paths: &mut Vec<PathBuf>,
        upper_path: &Path,
        work_path: &Path,
    ) -> Result<(), &'static str> {
        let last_component = path
            .iter()
            .last()
            .ok_or("Failed to get last component")?
            .to_string_lossy()
            .to_string();
        let backup_path = path
            .parent()
            .ok_or("Path has no parent")?
            .join(last_component + "~");

        // Add the backup path (original contents) to lower_paths
        lower_paths.push(backup_path.clone());
        let lower_paths_string =
            std::env::join_paths(lower_paths).map_err(|_| "Failed to join lower paths")?;
        let lower_paths_string = lower_paths_string.to_string_lossy();

        // Move path to backup
        let err = fs::rename(path, &backup_path).map_err(|_| "Failed to rename dir");
        if err.is_err() {
            error!("Failed to rename {:?} to {:?}", path, backup_path);
        }
        err?;

        // Recreate path so we can mount on it
        fs::create_dir(path).map_err(|_| "Failed to recreate dir")?;

        let mut cmd = Command::new("fuse-overlayfs");
        cmd.arg("-o");
        cmd.arg(format!(
            "lowerdir={},upperdir={},workdir={}",
            lower_paths_string,
            upper_path.display(),
            work_path.display()
        ));
        cmd.arg(path);

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(_) => return Err("Failed to spawn child"),
        };

        let status = match child.wait() {
            Ok(status) => status,
            Err(_) => return Err("Child failed"),
        };

        if !status.success() {
            return Err("Child failed");
        }

        self.mounted_paths.push(path.to_owned());

        info!("Mounted: {:?}", path);
        Ok(())
    }
    fn mount_all(&mut self) -> Result<(), &'static str> {
        let work_path = get_data_dir()?.join(".OverlayFS");
        verify_directory(&work_path)?;

        // Mount data
        let Some(install_path) = self.app.get_install_dir() else {
            return Err("Game not installed")
        };

        let data_path = install_path.join("Data");

        let mods_path = get_mods_dir()?;
        let mut mod_paths = get_active_mods(self.profile)?
            .into_iter()
            .map(|m| mods_path.join(m))
            .collect::<Vec<_>>();

        let override_path = get_overwrite_dir()?;

        self.mount_path(&data_path, &mut mod_paths, &override_path, &work_path)?;

        // Mount config
        let config_path = get_config_dir(self.app)?;
        let upper_path = get_data_dir()?.join("Configs");

        self.mount_path(&config_path, &mut Vec::new(), &upper_path, &work_path)?;

        // Mount appdata
        let appdata_path = get_appdata_dir(self.app)?;
        let upper_path = get_data_dir()?.join("Configs");

        self.mount_path(&appdata_path, &mut Vec::new(), &upper_path, &work_path)?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), &'static str> {
        self.mount_all()?;

        let result = self.app.run();

        info!("Game stopped");

        result
    }

    fn unmount_all(&mut self) -> Result<(), &'static str> {
        info!("Unmounting paths");
        if !self.mounted_paths.is_empty() {
            self.mounted_paths.retain(|path| {
                info!("--> {:?}", path);
                let mut cmd = Command::new("umount");
                cmd.arg(path);

                let mut child = match cmd.spawn() {
                    Ok(child) => child,
                    Err(_) => return true,
                };

                let status = match child.wait() {
                    Ok(status) => status,
                    Err(_) => return true,
                };

                if !status.success() {
                    return true;
                }

                let err = "Failed to restore path";
                let last_component = match path.iter().last() {
                    Some(component) => component,
                    None => {
                        error!("{}", err);
                        return false;
                    }
                }
                    .to_string_lossy()
                    .to_string();

                let backup_path = match path.parent() {
                    Some(path) => path,
                    None => {
                        error!("{}", err);
                        return false;
                    }
                }
                    .join(last_component + "~");

                if fs::rename(&backup_path, path).is_err() {
                    error!("{}", err);
                }

                false
            });

            if self.mounted_paths.is_empty() {
                Ok(())
            } else {
                error!("Failed to unmount: {:?}", self.mounted_paths);
                Err("Failed to unmount all paths")
            }
        } else {
            info!("No dirs to unmount.");
            Ok(())
        }
    }
}

impl<'a> Drop for AppLauncher<'a> {
    fn drop(&mut self) {
        info!("AppLauncher dropped");
        // Unmount directories
        if let Err(err) = self.unmount_all() {
            error!("{}", err);
            if !self.mounted_paths.is_empty() {
                for path in &self.mounted_paths {
                    error!("failed to unmount: {}", path.display());
                }
            }
        }
    }
}