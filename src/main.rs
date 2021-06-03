use clap::{crate_version, App, Arg, SubCommand};
use execute::{shell, Execute};
use log::{error, info, trace, warn};
use simplelog::TermLogger;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

static APP_NAME: &str = "Torygg";

static MODS_SUBDIR: &str = "Mods";
static OVERWRITE_SUBDIR: &str = "Overwrite";
static PROFILES_SUBDIR: &str = "Profiles";

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
        let archive_mount_dir = TempDir::new().unwrap();
        let mut archive_mount_path = archive_mount_dir.into_path();
        let mut mount_archive = shell(format!(
            "archivemount \"{}\" \"{}\"",
            archive_path.to_string_lossy(),
            archive_mount_path.to_string_lossy()
        ));

        mount_archive
            .execute()
            .map_err(|_| "Could not mount archive")?;

        // Detect if mod is contained within a subdirectory
        // and move it if it is
        let entries = fs::read_dir(&archive_mount_path)
            .map_err(|_| "Couldn't read dir")?
            .filter_map(|e| e.ok())
            .collect::<Vec<fs::DirEntry>>();
        if entries.len() == 1 {
            let path = entries[1].path();
            if path.is_dir() {
                archive_mount_path = path
            }
        }

        let mut create_squashfs = shell(format!(
            "mksquashfs \"{}\" \"{}\"",
            archive_mount_path.to_string_lossy(),
            get_mods_dir().unwrap().join(mod_name).to_string_lossy()
        ));

        create_squashfs
            .execute()
            .map_err(|_| "Could not create squashfs image")
            .map(|_| ())
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
            (!e.is_dir())
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
        let mod_dir = mount_mod(mod_name)?;
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
            .collect::<Vec<String>>()
            .join("\n");

        fs::write(get_profile_mods_dir(profile_name)?.join(mod_name), plugins)
            .map_err(|_| "Failed to write to profile's mods dir")
    }
}

fn deactivate_mod(profile_name: &str, mod_name: &str) -> Result<(), &'static str> {
    if !is_mod_installed(mod_name)? {
        Err("Mod not installed")
    } else if !is_mod_active(profile_name, mod_name)? {
        Err("Mod is not active")
    } else {
        fs::remove_file(get_profile_mods_dir(profile_name)?.join(mod_name))
            .map_err(|_| "Failed to remove mod file")
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
    Ok(PathBuf::from(
        std::env::var_os("TORYGG_SKYRIM_INSTALL_DIRECTORY")
            .ok_or("Environment variable 'TORYGG_SKYRIM_INSTALL_DIRECTORY' is not set!")?,
    ))
}

fn get_skyrim_data_dir() -> Result<PathBuf, &'static str> {
    Ok(get_skyrim_install_dir()?.join("Data"))
}

fn get_wine_user_dir() -> Result<PathBuf, &'static str> {
    Ok(PathBuf::from(
        std::env::var_os("TORYGG_USER_DIRECTORY")
            .ok_or("Environment variable 'TORYGG_USER_DIRECTORY' is not set!")?,
    ))
}

fn get_skyrim_config_dir() -> Result<PathBuf, &'static str> {
    Ok(get_wine_user_dir()?.join("My Documents/My Games/Skyrim Special Edition"))
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
    std::env::var("TORYGG_PROFILE").unwrap_or(String::from("Default"))
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

// Mount a directory using fuse-overlayfs
fn mount_directory(
    lower_dirs: &[PathBuf],
    upper_dir: &Path,
    work_dir: &Path,
    mount_dir: &Path,
) -> Result<(), &'static str> {
    let joined_paths = std::env::join_paths(lower_dirs).map_err(|_| "failed to join paths")?;
    let joined_paths = joined_paths.to_string_lossy();

    let mut command = shell(format!(
        "fuse-overlayfs -o lowerdir=\"{}\",upperdir=\"{}\",workdir=\"{}\" \"{}\"",
        joined_paths,
        upper_dir.display(),
        work_dir.display(),
        mount_dir.display(),
    ));

    match command.execute().map_err(|_| "Failed to execute command")? {
        Some(0) => Ok(()),
        _ => Err("Failed to mount overlayfs"),
    }
}

fn mount_mod(mod_name: &str) -> Result<PathBuf, &'static str> {
    let temp_dir = TempDir::new().map_err(|_| "Couldn't create temp dir")?;
    let path = temp_dir.into_path();

    let mut command = shell(format!(
        "squashfuse \"{}\" \"{}\"",
        get_mod_dir(mod_name)?.to_string_lossy(),
        path.to_string_lossy()
    ));

    command
        .execute()
        .map_err(|_| "failed to mount mount directory")?;

    Ok(path)
}

fn mount_skyrim_data_dir() -> Result<(), &'static str> {
    let skyrim_data_dir = get_skyrim_data_dir().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let mut lower_dirs = Vec::<PathBuf>::new();
    for m in get_active_mods(&get_active_profile())?.iter() {
        let squash_mount_dir = temp_dir.path().join(m);
        verify_directory(&squash_mount_dir).unwrap();

        let mut mount_squashfs = shell(format!(
            "squashfuse \"{}\" \"{}\"",
            get_mod_dir(m).unwrap().to_string_lossy(),
            squash_mount_dir.to_string_lossy()
        ));

        mount_squashfs
            .execute()
            .map_err(|_| "Failed to execute command")?;

        lower_dirs.push(squash_mount_dir);
    }

    //let mut lower_dirs: Vec<PathBuf> = mods.iter().rev().map(|m| temp_dir.path().join(m)).collect();
    lower_dirs.push(skyrim_data_dir.clone());

    let upper_dir = get_overwrite_dir().unwrap();
    verify_directory(&upper_dir)?;

    let work_dir = get_data_dir().unwrap().join(".OverlayFS");
    verify_directory(&work_dir)?;

    mount_directory(&lower_dirs, &upper_dir, &work_dir, &skyrim_data_dir)
}

fn mount_skyrim_configs_dir() -> Result<(), &'static str> {
    let skyrim_configs_dir = get_skyrim_config_dir()?;

    let override_config_dir = get_data_dir().unwrap().join("Configs");
    verify_directory(&override_config_dir)?;

    let work_dir = get_data_dir().unwrap().join(".OverlayFS");
    verify_directory(&work_dir)?;

    mount_directory(
        &[skyrim_configs_dir.clone()],
        &override_config_dir,
        &work_dir,
        &skyrim_configs_dir,
    )
}

fn unmount_directory(dir: &Path) -> Result<(), String> {
    let dir_string = format!("\"{}\"", dir.to_string_lossy());

    let mut command = shell(format!("umount {}", dir_string));
    match command.execute().unwrap() {
        Some(cmd_output) if cmd_output == 0 => Ok(()),
        _ => Err(format!(
            "Failed to umount overlayfs (maybe not mounted): {}",
            dir.display()
        )),
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
    )
    .unwrap();

    // Verify directories exist
    trace!("Verifying directories");

    if let Err(e) = verify_directory(&get_data_dir().unwrap()) {
        error!("{}", e);
        return;
    }

    if let Err(e) = verify_directory(&get_mods_dir().unwrap()) {
        error!("{}", e);
        return;
    }

    if let Err(e) = verify_directory(&get_overwrite_dir().unwrap()) {
        error!("{}", e);
        return;
    }

    if let Err(e) = verify_directory(&get_profiles_dir().unwrap()) {
        error!("{}", e);
        return;
    }

    let mut profiles = if let Ok(profiles) = get_profiles() {
        profiles
    } else {
        return;
    };

    if profiles.is_empty() {
        if let Err(e) = create_profile(&mut profiles, "Default") {
            error!("{}", e);
            return;
        }
    }

    let profile = &mut profiles[0];

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
        let mods = if let Ok(mods) = get_installed_mods() {
            mods
        } else {
            return;
        };

        for m in mods {
            print!("{}", m);
            println!("{}", {
                if let Ok(mod_active) = is_mod_active(profile, &m) {
                    if mod_active {
                        "*"
                    } else {
                        ""
                    }
                } else {
                    ""
                }
            });
        }
        return;
    }

    if let Some(_matches) = matches.subcommand_matches("mount") {
        info!("Mount.");

        if let Err(e) = mount_skyrim_data_dir() {
            error!("{}", e);
            return;
        }

        if let Err(e) = mount_skyrim_configs_dir() {
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

        return;
    }
}
