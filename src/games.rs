use std::collections::HashMap;
use crate::util;
use crate::wine::Prefix;
use log::info;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::error::ToryggError;

pub trait Game {
    fn get_install_dir(&self) -> Result<PathBuf, ToryggError>;
    fn get_executable(&self) -> Result<PathBuf, ToryggError>;
    fn get_wine_pfx(&self) -> Result<Prefix, ToryggError>;
    fn get_name(&self) -> &'static str;
    fn run(&self) -> Result<(), ToryggError>;
    fn get_wine_user_dir(&self) -> Result<PathBuf, ToryggError>;
    fn get_config_dir(&self) -> Result<PathBuf, ToryggError>;
    // Folder where profile Plugins.txt is kept
    fn get_appdata_dir(&self) -> Result<PathBuf, ToryggError>;
}

/// appid: Steam app id
/// name: Directory inside "$LIBRARY/steamapps/common" that the app is installed into
/// executable: game executable
/// mod_loader_executable: eg. skse64_loader.exe
#[derive(Debug, Clone)]
pub struct SteamApp {
    pub appid: usize,
    pub name: &'static str,
    pub executable: &'static str,
    pub mod_loader_executable: Option<&'static str>,
}

impl AsRef<SteamApp> for SteamApp {
    fn as_ref(&self) -> &SteamApp {
        self
    }
}

impl Serialize for SteamApp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(self.get_name())
    }
}

impl<'de> Deserialize<'de> for SteamApp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl<S> Game for S where S: AsRef<SteamApp> {
    fn get_install_dir(&self) -> Result<PathBuf, ToryggError> {
        let path = util::get_steam_library(self.as_ref())?
            .join("steamapps/common")
            .join(self.as_ref().name);

        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::DirectoryNotFound(path))
        }
    }
    fn get_executable(&self) -> Result<PathBuf, ToryggError> {
        let install_dir = self.get_install_dir()?;
        if let Some(mle) = self.as_ref().mod_loader_executable {
            let mle_path = install_dir.join(mle);
            if mle_path.exists() {
                return Ok(mle_path);
            }
        }

        Ok(install_dir.join(self.as_ref().executable))
    }

    fn get_wine_pfx(&self) -> Result<Prefix, ToryggError> {
        let path = util::get_steam_library(self.as_ref())?
            .join("steamapps/compatdata")
            .join(self.as_ref().appid.to_string())
            .join("pfx");

        if path.exists() {
            Err(ToryggError::PrefixNotFound)
        } else {
            Ok(Prefix::new(path))
        }
    }

    fn get_name(&self) -> &'static str {
        self.as_ref().name
    }

    fn run(&self) -> Result<(), ToryggError> {
        let install_dir = self.get_install_dir().unwrap();
        let executable = self.get_executable().unwrap();

        info!("Starting protontricks");
        let mut cmd = Command::new("protontricks");
        cmd.arg(self.as_ref().appid.to_string());
        cmd.arg("shell");
        cmd.stdin(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(_) => return Err(ToryggError::FailedToSpawnChild),
        };

        child
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
            )?;

        let status = match child.wait() {
            Ok(status) => status,
            Err(_) => return Err(ToryggError::ChildFailed),
        };

        if !status.success() {
            return Err(ToryggError::ChildFailed);
        }

        Ok(())
    }

    fn get_wine_user_dir(&self) -> Result<PathBuf, ToryggError> {
        // Prioritise a path specified via environment variable
        if let Some(str) = std::env::var_os("TORYGG_USER_DIRECTORY") {
            let path = PathBuf::from(str);
            return if path.exists() {
                Ok(path)
            } else {
                Err(ToryggError::DirectoryNotFound(path))
            }
        }

        let mut path = self.get_wine_pfx()?
            .pfx;
        path.push("drive_c/users");

        // When run through proton username is steamuser
        let steamuser = path.join("steamuser");
        if steamuser.exists() {
            return Ok(steamuser)
        }

        if let Some(current_user) =
            std::env::vars().collect::<HashMap<_, _>>().get("USER")
        {
            let user_dir = path.join(current_user);
            return if user_dir.exists() {
                Ok(user_dir)
            } else {
                Err(ToryggError::DirectoryNotFound(user_dir))
            }
        }

        Err(ToryggError::Other("wine user dir not found".to_owned()))
    }

    fn get_config_dir(&self) -> Result<PathBuf, ToryggError> {
        Ok(self.get_wine_user_dir()?.join(String::from("My Documents/My Games/") + self.get_name()))
    }

    // Folder where profile Plugins.txt is kept
    fn get_appdata_dir(&self) -> Result<PathBuf, ToryggError> {
        Ok(self.get_wine_user_dir()?
            .join(String::from("Local Settings/Application Data/") + self.get_name()))
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

impl std::str::FromStr for SteamApp {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        Ok(match s {
            s if s == SKYRIM.get_name() || s == "skyrim" => SKYRIM,
            s if s == SKYRIM_SPECIAL_EDITION.get_name() || s == "skyrimse" => SKYRIM_SPECIAL_EDITION,
            _ => anyhow::bail!("Unknown game \"{s}\""),
        })
    }
}
