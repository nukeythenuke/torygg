use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{error, info};
use crate::games::Game;
use crate::{config, Profile, util::verify_directory};

pub struct AppLauncher<'a> {
    app: &'static dyn Game,
    profile: &'a Profile,
    mounted_paths: Vec<PathBuf>,
}

impl<'a> AppLauncher<'a> {
    pub fn new(app: &'static dyn Game, profile: &'a Profile) -> Self {
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
        let work_path = config::get_data_dir()?.join(".OverlayFS");
        verify_directory(&work_path)?;

        // Mount data
        let Some(install_path) = self.app.get_install_dir() else {
            return Err("Game not installed")
        };

        let data_path = install_path.join("Data");

        let mods_path = config::get_mods_dir()?;
        let mut mod_paths = self.profile.get_enabled_mods()
            .into_iter()
            .map(|m| mods_path.join(m))
            .collect::<Vec<_>>();

        let override_path = config::get_overwrite_dir()?;

        self.mount_path(&data_path, &mut mod_paths, &override_path, &work_path)?;

        // Mount config
        let config_path = self.app.get_config_dir()?;
        let upper_path = config::get_data_dir()?.join("Configs");

        self.mount_path(&config_path, &mut Vec::new(), &upper_path, &work_path)?;

        // Mount appdata
        let appdata_path = self.app.get_appdata_dir()?;
        let upper_path = config::get_data_dir()?.join("Configs");

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