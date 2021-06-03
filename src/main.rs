use clap::{crate_version, App, Arg, SubCommand};
use execute::{shell, Execute};
use log::{error, info, trace, warn};
use simplelog::TermLogger;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

mod extract;

static APP_NAME: &str = "Torygg";

static MODS_SUBDIR: &str = "Mods";
static OVERWRITE_SUBDIR: &str = "Overwrite";
static PROFILES_SUBDIR: &str = "Profiles";

fn verify_directory(path: &Path) -> Result<(), String> {
    if path.exists() {
        if path.is_dir() {
            Ok(())
        } else {
            Err(format!("{} exists but is not a directory!", path.display()))
        }
    } else {
        match fs::create_dir(path) {
            Ok(_) => {
                info!("Created directory => {}", path.display());
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

fn install_mod_from_archive(archive_path: &Path, mod_name: &str) -> Result<(), String> {
    if !archive_path.exists() {
        Err(format!(
            "Archive \"{}\" does not exist!",
            archive_path.display()
        ))
    } else if get_installed_mods().contains(&mod_name.to_owned()) {
        Err("Mod already exists!".to_owned())
    } else {
        let archive_mount_dir = TempDir::new().unwrap();
        let mut archive_mount_path = archive_mount_dir.path().to_path_buf();
        let mut mount_archive = shell(format!(
            "archivemount \"{}\" \"{}\"",
            archive_path.to_string_lossy(),
            archive_mount_path.to_string_lossy()
        ));

        if let Err(e) = mount_archive.execute() {
            return Err(e.to_string());
        }

        // Detect if mod is contained within a subdirectory
        // and move it if it is
        let entrys = archive_mount_path.read_dir().unwrap();
        if entrys.count() == 1 {
            let entry = archive_mount_path
                .read_dir()
                .unwrap()
                .next()
                .unwrap()
                .unwrap();
            let path = entry.path();

            if path.is_dir() {
                archive_mount_path = path;
            }
        }

        let mut create_squashfs = shell(format!(
            "mksquashfs \"{}\" \"{}\"",
            archive_mount_path.to_string_lossy(),
            get_mods_dir().unwrap().join(mod_name).to_string_lossy()
        ));

        if let Err(e) = create_squashfs.execute() {
            return Err(e.to_string());
        }

        Ok(())
    }
}

fn create_mod(mod_name: &str) -> Result<(), String> {
    if !get_installed_mods().contains(&mod_name.to_owned()) {
        verify_directory(&get_mods_dir().unwrap().join(mod_name))?;
        Ok(())
    } else {
        Err("Mod with same name already exists!".to_owned())
    }
}

fn uninstall_mod(mod_name: &str) -> Result<(), String> {
    if get_installed_mods().contains(&mod_name.to_owned()) {
        for p in get_profiles() {
            if deactivate_mod(&p, mod_name).is_err() {};
        }

        if let Err(e) = std::fs::remove_file(get_mod_dir(mod_name).unwrap()) {
            Err(e.to_string())
        } else {
            Ok(())
        }
    } else {
        Err("Mod not installed".to_owned())
    }
}

/*
* Copy a file from one mods installation folder to another
*
* from: mod to copy the file from
* to: mod to copy the file to
* dir: relative path to the file to copy
*/
fn copy_file_between_mods(from: &str, to: &str, file: &Path) -> Result<(), String> {
    let from_dir = match get_mod_dir(from) {
        Some(from_dir) => from_dir,
        None => return Err(format!("Mod \"{}\" does not exist", from)),
    };

    let to_dir = match get_mod_dir(to) {
        Some(to_dir) => to_dir,
        None => return Err(format!("Mod \"{}\" does not exist", to)),
    };

    if let Err(e) = std::fs::copy(from_dir.join(file), to_dir.join(file)) {
        error!(
            "Failed to move \"{}\" from \"{}\" to \"{}\"",
            file.display(),
            from_dir.display(),
            to_dir.display()
        );
        return Err(e.to_string());
    }

    Ok(())
}

/*
* Copy a directory from one mods installation folder to another
*
* from: mod to copy the directory from
* to: mod to copy the directory to
* dir: relative path to the directory to copy
*/
fn copy_dir_between_mods(from: &str, to: &str, dir: &Path) -> Result<(), String> {
    let from_dir = if let Some(from_dir) = get_mod_dir(from) {
        from_dir
    } else {
        return Err(format!("Mod \"{}\" does not exist", from));
    };

    let to_dir = if let Some(to_dir) = get_mod_dir(to) {
        to_dir
    } else {
        return Err(format!("Mod \"{}\" does not exist", from));
    };

    for f in WalkDir::new(from_dir.join(dir)).min_depth(1).into_iter() {
        let f = f.unwrap();
        let p = f.path();

        let suffix = p.strip_prefix(&from_dir).unwrap();

        if p.is_dir() {
            info!(
                "Copy dir \"{}\" from \"{}\" to \"{}\"",
                suffix.display(),
                from,
                to
            );
            verify_directory(&to_dir.join(suffix))?;
            copy_dir_between_mods(from, to, suffix)?;
        } else {
            info!(
                "Copy file \"{}\" from \"{}\" to \"{}\"",
                suffix.display(),
                from,
                to
            );
            copy_file_between_mods(from, to, suffix)?;
        }
    }

    Ok(())
}

/*
* Remove a file from a mods installation folder
*
* mod_name: name of the mod to remove a file from
* dir: relative path to the file to remove
*/
fn remove_file_from_mod(mod_name: &str, file: &Path) -> Result<(), String> {
    if let Some(mod_dir) = get_mod_dir(mod_name) {
        if let Err(e) = std::fs::remove_file(mod_dir.join(file)) {
            Err(e.to_string())
        } else {
            Ok(())
        }
    } else {
        Err(format!("Mod \"{}\" does not exist", mod_name))
    }
}

/*
* Remove a directory from a mods installation folder
*
* mod_name: name of the mod to remove a directory from
* dir: relative path to the directory to remove
*/
fn remove_dir_from_mod(mod_name: &str, dir: &Path) -> Result<(), String> {
    if let Some(mod_dir) = get_mod_dir(mod_name) {
        if let Err(e) = std::fs::remove_dir_all(mod_dir.join(dir)) {
            Err(e.to_string())
        } else {
            Ok(())
        }
    } else {
        Err(format!("Mod \"{}\" does not exist", mod_name))
    }
}

/*
* Move collection of files / directories from one mod to another
*
* from: mod to move the file / directory from
* to: mod to move the file / directory to
* paths: slice containing relative paths to files/ directories to move
*/
fn move_files_between_mods(from: &str, to: &str, paths: &[&Path]) -> Result<(), String> {
    for path in paths {
        let is_dir = if let Some(from_dir) = get_mod_dir(from) {
            from_dir.join(path).is_dir()
        } else {
            return Err(format!(
                "\"{}\" does not exists in \"{}\"",
                path.display(),
                from
            ));
        };

        if is_dir {
            info!(
                "Move dir \"{}\" form \"{}\" to \"{}\"",
                path.display(),
                from,
                to
            );
            copy_dir_between_mods(from, to, path)?;
            remove_dir_from_mod(from, path)?;
        } else {
            info!(
                "Move file \"{}\" form \"{}\" to \"{}\"",
                path.display(),
                from,
                to
            );
            copy_file_between_mods(from, to, path)?;
            remove_file_from_mod(from, path)?;
        }
    }

    Ok(())
}

fn get_mod_dir(mod_name: &str) -> Option<PathBuf> {
    let dir = get_mods_dir()?.join(mod_name);

    if dir.exists() {
        Some(dir)
    } else {
        None
    }
}

fn get_installed_mods() -> Vec<String> {
    let mut mods = Vec::<String>::new();
    for m in WalkDir::new(get_mods_dir().unwrap().as_path())
        .min_depth(1)
        .max_depth(1)
    {
        let m = m.unwrap();
        let path = m.path();
        if !path.is_dir() {
            if let Some(fs) = path.file_stem() {
                mods.push(fs.to_string_lossy().to_string());
            }
        }
    }

    mods
}

fn is_mod_installed(mod_name: &str) -> bool {
    get_installed_mods().contains(&mod_name.to_owned())
}

fn get_profiles() -> Vec<String> {
    let mut profiles = Vec::<String>::new();
    for p in WalkDir::new(get_profiles_dir().unwrap())
        .min_depth(1)
        .max_depth(1)
    {
        let path = match p {
            Ok(p) => p.into_path(),
            Err(_) => break,
        };

        if path.is_dir() {
            if let Some(fs) = path.file_stem() {
                info!("Found profile \"{:?}\"", fs);
                profiles.push(fs.to_string_lossy().to_string());
            }
        }
    }

    profiles
}

fn create_profile(profiles: &mut Vec<String>, profile_name: &str) -> Result<(), String> {
    let path = get_profiles_dir().unwrap().join(profile_name);
    if path.exists() {
        return Err(format!("Profile \"{}\" already exists!", profile_name));
    }
    verify_directory(path.as_path())?;

    profiles.push(profile_name.to_owned());

    Ok(())
}

fn activate_mod(profile_name: &str, mod_name: &str) -> Result<(), &'static str> {
    if is_mod_installed(mod_name) && !is_mod_active(profile_name, mod_name) {
        // Discover plugin files and store their names
        let mut plugins = String::new();
        let mod_dir = mount_mod(mod_name)?;

        for entry in WalkDir::new(mod_dir).min_depth(1).max_depth(1) {
            let entry = entry.map_err(|_| "Invalid entry")?;
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            if let Some(extension) = path.extension() {
                let extension = extension
                    .to_str()
                    .ok_or("Can't convert extension to &str")?;
                match extension {
                    "esp" | "esm" | "esl" => {
                        plugins.push_str(
                            path.file_name()
                                .ok_or("No file name")?
                                .to_str()
                                .ok_or("Can't convert extension to &str")?,
                        );
                        plugins.push('\n')
                    }
                    _ => (),
                }
            }
        }

        std::fs::write(get_profile_mods_dir(profile_name).join(mod_name), plugins)
            .map_err(|_| "Couldn't write to profile mods dir")
    } else {
        Err("Mod not installed or is already active")
    }
}

fn deactivate_mod(profile_name: &str, mod_name: &str) -> Result<(), String> {
    if is_mod_installed(mod_name) && is_mod_active(profile_name, mod_name) {
        if let Err(e) = std::fs::remove_file(get_profile_mods_dir(profile_name).join(mod_name)) {
            Err(e.to_string())
        } else {
            Ok(())
        }
    } else {
        Err("Mod not installed or is already inactive".to_owned())
    }
}

fn get_active_mods(profile_name: &str) -> Vec<String> {
    let mut mods = Vec::<String>::new();
    for e in WalkDir::new(get_profile_mods_dir(profile_name))
        .min_depth(1)
        .max_depth(1)
    {
        let e = e.unwrap();
        let path = e.path();
        if path.is_dir() {
            continue;
        }
        if let Some(fs) = path.file_stem() {
            mods.push(fs.to_string_lossy().to_string());
        }
    }
    mods
}

fn is_mod_active(profile_name: &str, mod_name: &str) -> bool {
    get_active_mods(profile_name).contains(&mod_name.to_owned())
}

fn get_skyrim_install_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os(
        "TORYGG_SKYRIM_INSTALL_DIRECTORY",
    )?))
}

fn get_skyrim_data_dir() -> Option<PathBuf> {
    Some(get_skyrim_install_dir()?.join("Data"))
}

fn get_wine_user_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("TORYGG_USER_DIRECTORY")?))
}

fn get_skyrim_config_dir() -> Option<PathBuf> {
    Some(get_wine_user_dir()?.join("My Documents/My Games/Skyrim Special Edition"))
}

fn get_data_dir() -> Option<PathBuf> {
    let dir = dirs::data_dir()?.join(APP_NAME);
    verify_directory(&dir).ok()?;
    Some(dir)
}

fn get_mods_dir() -> Option<PathBuf> {
    let dir = get_data_dir()?.join(MODS_SUBDIR);
    verify_directory(&dir).ok()?;
    Some(dir)
}

fn get_overwrite_dir() -> Option<PathBuf> {
    let dir = get_data_dir()?.join(OVERWRITE_SUBDIR);
    verify_directory(&dir).ok()?;
    Some(dir)
}

fn get_active_profile() -> String {
    match std::env::var("TORYGG_PROFILE") {
        Ok(p) => p,
        Err(_) => String::from("Default"),
    }
}

fn get_profiles_dir() -> Option<PathBuf> {
    let dir = get_data_dir()?.join(PROFILES_SUBDIR);
    verify_directory(&dir).ok()?;
    Some(dir)
}

fn get_profile_dir(profile_name: &str) -> PathBuf {
    let dir = get_profiles_dir().unwrap().join(profile_name);
    verify_directory(&dir).unwrap();
    dir
}

fn get_profile_mods_dir(profile_name: &str) -> PathBuf {
    let dir = get_profile_dir(&profile_name).join("Mods");
    verify_directory(&dir).unwrap();
    dir
}

// Mount a directory using fuse-overlayfs
fn mount_directory(
    lower_dirs: &[PathBuf],
    upper_dir: &Path,
    work_dir: &Path,
    mount_dir: &Path,
) -> Result<(), String> {
    let mut lower_dirs_string = String::from('\"');
    let joined_paths = match std::env::join_paths(lower_dirs) {
        Ok(s) => s,
        Err(e) => return Err(e.to_string()),
    };
    lower_dirs_string.push_str(&joined_paths.to_string_lossy());
    lower_dirs_string.push('\"');

    let upper_dir_string = format!("\"{}\"", upper_dir.to_string_lossy());
    let work_dir_string = format!("\"{}\"", work_dir.to_string_lossy());
    let mount_dir_string = format!("\"{}\"", mount_dir.to_string_lossy());

    let mut command = shell(format!(
        "fuse-overlayfs -o lowerdir={},upperdir={},workdir={} {}",
        lower_dirs_string, upper_dir_string, work_dir_string, mount_dir_string,
    ));

    match command.execute().unwrap() {
        Some(cmd_output) if cmd_output == 0 => Ok(()),
        _ => Err(format!(
            "Failed to mount overlayfs (maybe already mounted): {}",
            mount_dir.display()
        )),
    }
}

fn mount_mod(mod_name: &str) -> Result<PathBuf, &'static str> {
    let temp_dir = TempDir::new().map_err(|_| "Couldn't create temp dir")?;
    let path = temp_dir.into_path();

    let mut command = shell(format!(
        "squashfuse \"{}\" \"{}\"",
        get_mod_dir(mod_name)
            .ok_or("mod image not found")?
            .to_string_lossy(),
        path.to_string_lossy()
    ));

    command
        .execute()
        .map_err(|_| "failed to mount mount directory")?;

    Ok(path)
}

fn mount_skyrim_data_dir() -> Result<(), String> {
    let skyrim_data_dir = get_skyrim_data_dir().unwrap();
    let temp_dir = TempDir::new().unwrap();

    let mut lower_dirs = Vec::<PathBuf>::new();
    for m in get_active_mods(&get_active_profile()).iter() {
        let squash_mount_dir = temp_dir.path().join(m);
        verify_directory(&squash_mount_dir).unwrap();

        let mut mount_squashfs = shell(format!(
            "squashfuse \"{}\" \"{}\"",
            get_mod_dir(m).unwrap().to_string_lossy(),
            squash_mount_dir.to_string_lossy()
        ));

        if let Err(e) = mount_squashfs.execute() {
            return Err(e.to_string());
        }

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

fn mount_skyrim_configs_dir() -> Result<(), String> {
    let skyrim_configs_dir = get_skyrim_config_dir().ok_or("Could not find skyrim config dir")?;

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
    match get_data_dir() {
        Some(dir) => info!("Data directory => {}", dir.display()),
        None => {
            error!("Could not find data dir!");
            return;
        }
    }

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

    let mut profiles = get_profiles();

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
        for m in &get_installed_mods() {
            print!("{}", m);
            println!("{}", if is_mod_active(profile, m) { "*" } else { "" });
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
