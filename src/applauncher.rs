use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use log::{error, info};
use crate::{config, profile::Profile, util::verify_directory};
use crate::error::ToryggError;
use crate::games::Game;

pub struct AppLauncher<'a> {
    profile: &'a Profile,
    mounted_paths: Vec<PathBuf>,
}

impl<'a> AppLauncher<'a> {
    pub fn new(profile: &'a Profile) -> Self {
        AppLauncher {
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
    ) -> Result<(), ToryggError> {
        let dir_name = path.file_name()
            .ok_or(ToryggError::Other("could not get folder name".to_owned()))?
            .to_string_lossy().to_string();
        let backup_path = path
            .parent()
            .ok_or(ToryggError::Other("path has no parent".to_owned()))?
            .join(dir_name + "~");

        // Add the backup path (original contents) to lower_paths
        lower_paths.push(backup_path.clone());
        let lower_paths_string =
            std::env::join_paths(lower_paths).map_err(|_| ToryggError::Other("failed to join lower paths".to_owned()))?;
        let lower_paths_string = lower_paths_string.to_string_lossy();

        // Move path to backup
        fs::rename(path, &backup_path)?;

        // Recreate path so we can mount on it
        fs::create_dir(path)?;

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
            Err(_) => return Err(ToryggError::FailedToSpawnChild),
        };

        let status = match child.wait() {
            Ok(status) => status,
            Err(_) => return Err(ToryggError::ChildFailed),
        };

        if !status.success() {
            return Err(ToryggError::ChildFailed);
        }

        self.mounted_paths.push(path.to_owned());

        info!("Mounted: {:?}", path);
        Ok(())
    }

    fn mount_all(&mut self) -> Result<(), ToryggError> {
        let work_path = config::get_data_dir().join(".OverlayFS");
        verify_directory(&work_path)?;

        // Mount data
        let install_path = self.profile.get_game().get_install_dir()?;

        let data_path = install_path.join("Data");

        let mut mod_paths = match self.profile.get_enabled_mods() {
            Some(mods) => {
                let mods_path = self.profile.get_mods_dir()?;
                mods.into_iter()
                    .map(|m| mods_path.join(m))
                    .collect::<Vec<_>>()
            }
            None => Vec::new()
        };


        let override_path = self.profile.get_overwrite_dir()?;

        self.mount_path(&data_path, &mut mod_paths, &override_path, &work_path)?;

        // Mount config
        let config_path = self.profile.get_game().get_config_dir()?;
        let upper_path = config::get_data_dir().join("Configs");

        self.mount_path(&config_path, &mut Vec::new(), &upper_path, &work_path)?;

        // Mount appdata
        let appdata_path = self.profile.get_game().get_appdata_dir()?;
        let upper_path = config::get_data_dir().join("Configs");

        self.mount_path(&appdata_path, &mut Vec::new(), &upper_path, &work_path)?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), ToryggError> {
        self.mount_all()?;

        let result = self.profile.get_game().run();

        info!("Game stopped");

        result
    }

    fn unmount_all(&mut self) -> Result<(), ToryggError> {
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
                Err(ToryggError::Other("Failed to unmount directories".to_owned()))
            }
        } else {
            info!("No dirs to unmount.");
            Ok(())
        }
    }
}

impl Drop for AppLauncher<'_> {
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