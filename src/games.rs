use crate::util;
use crate::wine::Prefix;
use log::info;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub trait Game {
    fn get_install_dir(&self) -> Option<PathBuf>;
    fn get_executable(&self) -> Option<PathBuf>;
    fn get_wine_pfx(&self) -> Option<Prefix>;
    fn get_name(&self) -> &'static str;
    fn run(&self) -> Result<(), &'static str>;
}

/// appid: Steam app id
/// name: Directory inside "$LIBRARY/steamapps/common" that the app is installed into
/// executable: game executable
/// mod_loader_executable: eg. skse64_loader.exe
#[derive(Debug)]
pub struct SteamApp {
    pub appid: usize,
    pub name: &'static str,
    pub executable: &'static str,
    pub mod_loader_executable: Option<&'static str>,
}

impl Game for SteamApp {
    fn get_install_dir(&self) -> Option<PathBuf> {
        let path = util::get_steam_library(self)?
            .join("steamapps/common")
            .join(self.name);

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
        self.name
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
    name: "Skyrim",
    executable: "Skyrim.exe",
    mod_loader_executable: None,
};
pub const SKYRIM_SPECIAL_EDITION: SteamApp = SteamApp {
    appid: 489830,
    name: "Skyrim Special Edition",
    executable: "SkyrimSE.exe",
    mod_loader_executable: Some("skse64_loader.exe"),
};

impl std::str::FromStr for &SteamApp {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        Ok(match s {
            "skyrim" => &SKYRIM,
            "skyrimse" => &SKYRIM_SPECIAL_EDITION,
            _ => anyhow::bail!("Unknown game \"{s}\""),
        })
    }
}
