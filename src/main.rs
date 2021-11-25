use clap::{crate_version, App, Arg, SubCommand};
use execute::{shell, Execute};
use log::{error, info, warn};
use simplelog::TermLogger;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;

static APP_NAME: &str = "torygg";

static MODS_SUBDIR: &str = "Mods";
static OVERWRITE_SUBDIR: &str = "Overwrite";
static PROFILES_SUBDIR: &str = "Profiles";

mod util {
    use std::{fs::File, iter::FromIterator, path::PathBuf};

    pub mod apps {
        /// appid: Steam app id
        /// install_dir: Directory inside "$LIBRARY/steamapps/common" that the app is installed into
        #[derive(Debug, Clone, Copy)]
        pub struct SteamApp {
            pub appid: isize,
            pub install_dir: &'static str,
        }

        pub const SKYRIM: SteamApp = SteamApp {
            appid: 72850,
            install_dir: "Skyrim",
        };
        pub const SKYRIM_SPECIAL_EDITION: SteamApp = SteamApp {
            appid: 489830,
            install_dir: "Skyrim Special Edition",
        };
    }

    pub fn get_libraryfolders_vdf() -> PathBuf {
        PathBuf::from(std::env::var("HOME").unwrap()).join(".steam/root/config/libraryfolders.vdf")
    }

    fn get_steam_library(app: apps::SteamApp) -> Option<PathBuf> {
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

    pub fn get_install_dir(app: apps::SteamApp) -> Option<PathBuf> {
        let path = get_steam_library(app)?
            .join("steamapps/common")
            .join(app.install_dir);

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_wine_prefix(app: apps::SteamApp) -> Option<PathBuf> {
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

fn verify_directory(path: &Path) -> Result<(), &'static str> {
    if path.exists() {
        path.is_dir()
            .then(|| ())
            .ok_or("path exists but is not a directory")
    } else {
        fs::create_dir(path).map_err(|_| "Couldn't create directory")
    }
}

fn install_mod_from_archive(archive_path: &Path, mod_name: &str) -> Result<(), &'static str> {
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

        let status = command
            .status()
            .map_err(|_| "Unable to extract archive")?;
        if !status.success() {
            return Err("Unable to extract archive");
        }

        // Detect if mod is contained within a subdirectory
        // and move it if it is
        let mut mod_root = archive_extract_path.clone();
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
        for entry in WalkDir::new(&mod_root).min_depth(1).into_iter().filter_map(|e| e.ok()) {
            let from = entry.path();
            let relative_path = from.strip_prefix(&mod_root).unwrap();
            let to = install_path.join(relative_path);

            if from.is_dir() {
                std::fs::create_dir(to).unwrap();
            } else {
                std::fs::copy(from, to).unwrap();
            }
        }

        Ok(())
    }
}

fn create_mod(mod_name: &str) -> Result<(), &'static str> {
    if !is_mod_installed(mod_name)? {
        verify_directory(&get_mods_dir().unwrap().join(mod_name))
    } else {
        Err("Mod with same name already exists!")
    }
}

fn uninstall_mod(mod_name: &str) -> Result<(), &'static str> {
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

fn get_installed_mods() -> Result<Vec<String>, &'static str> {
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

fn get_profiles() -> Result<Vec<String>, &'static str> {
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

fn create_profile(profiles: &mut Vec<String>, profile_name: &str) -> Result<(), &'static str> {
    let path = get_profiles_dir()?.join(profile_name);
    if path.exists() {
        Err("Profile already exists!")
    } else {
        verify_directory(&path)?;
        profiles.push(profile_name.to_owned());
        Ok(())
    }
}

fn activate_mod(profile_name: &str, mod_name: &str) -> Result<(), &'static str> {
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

fn deactivate_mod(profile_name: &str, mod_name: &str) -> Result<(), &'static str> {
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

fn is_mod_active(profile_name: &str, mod_name: &str) -> Result<bool, &'static str> {
    Ok(get_active_mods(profile_name)?.contains(&mod_name.to_owned()))
}

fn get_skyrim_install_dir() -> Result<PathBuf, &'static str> {
    match std::env::var_os("TORYGG_SKYRIM_INSTALL_DIRECTORY") {
        Some(str) => {
            let path = PathBuf::from(str);
            if path.exists() {
                Ok(path)
            } else {
                Err("specified path does not exist")
            }
        }
        None => util::get_install_dir(util::apps::SKYRIM_SPECIAL_EDITION)
            .ok_or("skyrim install dir not found"),
    }
}

fn get_skyrim_data_dir() -> Result<PathBuf, &'static str> {
    Ok(get_skyrim_install_dir()?.join("Data"))
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
            let mut path = util::get_wine_prefix(util::apps::SKYRIM_SPECIAL_EDITION)
                .ok_or("skyrim install dir not found")?;
            path.push("drive_c/users");
            let steamuser = path.join("steamuser");
            if steamuser.exists() {
                Ok(steamuser)
            } else {
                if let Some(current_user) = std::env::vars().collect::<HashMap<_, _>>().get("USER")
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
}

fn get_skyrim_config_dir() -> Result<PathBuf, &'static str> {
    Ok(get_wine_user_dir()?.join("My Documents/My Games/Skyrim Special Edition"))
}

// Folder where profile Plugins.txt is kept
fn get_skyrim_appdata_dir() -> Result<PathBuf, &'static str> {
    Ok(get_wine_user_dir()?.join("Local Settings/Application Data/Skyrim Special Edition"))
}

fn get_data_dir() -> Result<PathBuf, &'static str> {
    let dir = dirs::data_dir()
        .ok_or("Could not find torygg's data dir")?
        .join(APP_NAME);
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_mods_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(MODS_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_overwrite_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(OVERWRITE_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_active_profile() -> String {
    std::env::var("TORYGG_PROFILE").unwrap_or_else(|_| String::from("Default"))
}

fn get_profiles_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(PROFILES_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_profile_dir(profile_name: &str) -> Result<PathBuf, &'static str> {
    let dir = get_profiles_dir()?.join(profile_name);
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_profile_mods_dir(profile_name: &str) -> Result<PathBuf, &'static str> {
    let dir = get_profile_dir(&profile_name)?.join("Mods");
    verify_directory(&dir)?;
    Ok(dir)
}

fn get_profile_appdata_dir(profile_name: &str) -> Result<PathBuf, &'static str> {
    let dir = get_profile_dir(profile_name)?.join("AppData");
    verify_directory(&dir)?;
    Ok(dir)
}

// Mount a directory using fuse-overlayfs
fn mount_directory(
    lower_dirs: &[PathBuf],
    upper_dir: &Path,
    work_dir: &Path,
    mount_dir: &Path,
) -> Result<(), &'static str> {
    let joined_paths = std::env::join_paths(lower_dirs).map_err(|_| "failed to join paths")?;
    let joined_paths = joined_paths.to_string_lossy();

    info!("lowerdirs={:?}", lower_dirs);
    info!("upperdir={:?}", upper_dir);
    info!("work_dir={:?}", work_dir);
    info!("mount_dir={:?}", mount_dir);

    let mut command = shell(format!(
        "fuse-overlayfs -o lowerdir=\"{}\",upperdir=\"{}\",workdir=\"{}\" \"{}\"",
        joined_paths,
        upper_dir.display(),
        work_dir.display(),
        mount_dir.display(),
    ));

    info!("command={:?}", command);

    let status = command.status().map_err(|_| "Failed to execute command")?;
    if status.success() {
        Ok(())
    } else {
        Err("Failed to mount overlayfs")
    }
}

fn mount_skyrim_data_dir() -> Result<(), &'static str> {
    let skyrim_data_dir = get_skyrim_data_dir()?;

    let mods_dir = get_mods_dir()?;
    let mut lower_dirs = get_active_mods(&get_active_profile())?
        .into_iter()
        .map(|m| mods_dir.join(m))
        .collect::<Vec<PathBuf>>();
    lower_dirs.push(skyrim_data_dir.clone());

    let upper_dir = get_overwrite_dir()?;
    verify_directory(&upper_dir)?;

    let work_dir = get_data_dir()?.join(".OverlayFS");
    verify_directory(&work_dir)?;

    mount_directory(&lower_dirs, &upper_dir, &work_dir, &skyrim_data_dir)
}

fn mount_skyrim_configs_dir() -> Result<(), &'static str> {
    let skyrim_configs_dir = get_skyrim_config_dir()?;

    let override_config_dir = get_data_dir()?.join("Configs");
    verify_directory(&override_config_dir)?;

    let work_dir = get_data_dir()?.join(".OverlayFS");
    verify_directory(&work_dir)?;

    mount_directory(
        &[skyrim_configs_dir.clone()],
        &override_config_dir,
        &work_dir,
        &skyrim_configs_dir,
    )
}

fn mount_skyrim_appdata_dir() -> Result<(), &'static str> {
    let skyrim_appdata_dir = get_skyrim_appdata_dir()?;

    let override_appdata_dir = get_profile_appdata_dir(&get_active_profile())?;

    let work_dir = get_data_dir()?.join(".OverlayFS");
    verify_directory(&work_dir)?;

    mount_directory(
        &[skyrim_appdata_dir.clone()],
        &override_appdata_dir,
        &work_dir,
        &skyrim_appdata_dir,
    )
}

fn unmount_directory(dir: &Path) -> Result<(), &'static str> {
    let dir_string = format!("\"{}\"", dir.to_string_lossy());

    let mut command = shell(format!("umount {}", dir_string));
    match command.execute().map_err(|_| "Failed to execute command")? {
        Some(0) => Ok(()),
        _ => Err("Failed to umount overlayfs"),
    }
}

fn main() {
    let matches = App::new(APP_NAME)
        .version(crate_version!())
        .author("Norman McKeown")
        .about("A mod manager for Skyrim Special Edition on linux")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Enable verbose output"),
        )
        .subcommand(SubCommand::with_name("mods").about("List installed / active mods"))
        .subcommand(
            SubCommand::with_name("install")
                .about("Install a mod from an archive")
                .arg(
                    Arg::with_name("ARCHIVE")
                        .help("Mod archive to install")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("NAME")
                        .help("Set the name of the installed mod")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("uninstall")
                .about("Uninstall the given mod")
                .arg(
                    Arg::with_name("name")
                        .help("Mod to uninstall")
                        .value_name("NAME")
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("activate")
                .about("Activate the given mod")
                .arg(
                    Arg::with_name("NAME")
                        .help("Mod to activate")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("deactivate")
                .about("Deactivate the given mod")
                .arg(
                    Arg::with_name("NAME")
                        .help("Mod to activate")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("create")
                .about("Create a new mod with the given name")
                .arg(
                    Arg::with_name("NAME")
                        .help("Mod to create")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(SubCommand::with_name("overwrite").about("List contents of overwite directory"))
        .subcommand(
            SubCommand::with_name("mount").about("Mount skyrim data, config & save directories"),
        )
        .subcommand(
            SubCommand::with_name("umount").about("Umount skyrim data, config & save directories"),
        )
        .get_matches();

    TermLogger::init(
        if matches.is_present("verbose") {
            simplelog::LevelFilter::Info
        } else {
            simplelog::LevelFilter::Warn
        },
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    // Verify directories exist
    if let Err(e) = || -> Result<(), &'static str> {
        verify_directory(&get_data_dir()?)?;
        verify_directory(&get_mods_dir()?)?;
        verify_directory(&get_overwrite_dir()?)?;
        verify_directory(&get_profiles_dir()?)
    }() {
        error!("{}", e);
        return;
    }

    // Get profiles, create a default profile if none exist
    let profiles = match || -> Result<Vec<String>, &'static str> {
        let mut profiles = get_profiles()?;
        if profiles.is_empty() {
            create_profile(&mut profiles, "Default")?;
        }

        Ok(profiles)
    }() {
        Ok(profiles) => profiles,
        Err(e) => {
            error!("{}", e);
            return;
        }
    };

    let profile = &profiles[0];

    if let Some(matches) = matches.subcommand_matches("install") {
        info!("Install.");
        let archive_path = Path::new(matches.value_of("ARCHIVE").unwrap());
        info!("Archive => {:?}", archive_path);
        let mod_name = match matches.value_of("NAME") {
            Some(mod_name) => mod_name.to_string(),
            _ => archive_path
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        };
        info!("Mod name => {}", mod_name);

        if let Err(e) = install_mod_from_archive(Path::new(archive_path), &mod_name) {
            error!("{}", e);
            return;
        }

        return;
    }

    if let Some(matches) = matches.subcommand_matches("uninstall") {
        info!("Uninstall.");

        let mod_name = matches.value_of("name").unwrap().to_owned();

        if let Err(e) = uninstall_mod(&mod_name) {
            error!("{}", e);
        }

        return;
    }

    if let Some(matches) = matches.subcommand_matches("activate") {
        info!("Activate.");

        let mod_name = matches.value_of("NAME").unwrap().to_owned();
        if let Err(e) = activate_mod(&get_active_profile(), &mod_name) {
            error!("{}", e);
        }

        return;
    }

    if let Some(matches) = matches.subcommand_matches("deactivate") {
        info!("Deactivate.");

        let mod_name = matches.value_of("NAME").unwrap().to_owned();
        if let Err(e) = deactivate_mod(&get_active_profile(), &mod_name) {
            error!("{}", e);
        }

        return;
    }

    if let Some(matches) = matches.subcommand_matches("create") {
        info!("Create.");

        let mod_name = matches.value_of("NAME").unwrap().to_owned();
        if let Err(e) = create_mod(&mod_name) {
            error!("{}", e);
        }

        return;
    }

    if let Some(_matches) = matches.subcommand_matches("overwrite") {
        for e in WalkDir::new(get_overwrite_dir().unwrap()).min_depth(1) {
            println!("{}", e.unwrap().path().display());
        }

        return;
    }

    if let Some(_matches) = matches.subcommand_matches("mods") {
        println!("Mods");
        let mods = match get_installed_mods() {
            Ok(mods) => mods.into_iter(),
            Err(e) => {
                error!("{}", e);
                return;
            }
        };

        let statuses = mods.clone().map(|m| match is_mod_active(profile, &m) {
            Ok(enabled) => {
                if enabled {
                    "*"
                } else {
                    ""
                }
            }
            Err(_) => "",
        });

        let combined = mods.zip(statuses);

        for m in combined {
            println!("{}{}", m.1, m.0)
        }

        return;
    }

    if let Some(_matches) = matches.subcommand_matches("mount") {
        info!("Mount.");

        if let Err(e) = || -> Result<(), &'static str> {
            info!("Mounting data dir");
            mount_skyrim_data_dir()?;
            info!("Mounting configs dir");
            mount_skyrim_configs_dir()?;
            info!("Mounting appdata dir");
            mount_skyrim_appdata_dir()
        }() {
            error!("{}", e);
            return;
        }

        return;
    }

    if let Some(_matches) = matches.subcommand_matches("umount") {
        info!("Umount.");

        if let Err(e) = unmount_directory(&get_skyrim_data_dir().unwrap()) {
            warn!("{}", e);
        }

        if let Err(e) = unmount_directory(&get_skyrim_config_dir().unwrap()) {
            warn!("{}", e);
        }

        if let Err(e) = unmount_directory(&get_skyrim_appdata_dir().unwrap()) {
            warn!("{}", e);
        }

        return;
    }
}
