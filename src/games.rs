use std::collections::HashMap;
use crate::util;
use std::path::PathBuf;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::error::ToryggError;

pub trait Game {
    /// The games installation directory
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    fn install_dir(&self) -> Result<PathBuf, ToryggError>;

    /// The wine prefix associated with the game
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    fn wine_pfx(&self) -> Result<PathBuf, ToryggError>;

    /// The name of the game
    fn name(&self) -> &'static str;

    /// The user windows user directory in the wine prefix
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    fn wine_user_dir(&self) -> Result<PathBuf, ToryggError>;

    /// The games config directory
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    fn config_dir(&self) -> Result<PathBuf, ToryggError> {
        let path = self.wine_user_dir()?.join(String::from("My Documents/My Games/") + self.name());
        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::DirectoryNotFound(path))
        }
    }

    // Directory in which Plugins.txt is kept
    /// The games appdata directory
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    fn appdata_dir(&self) -> Result<PathBuf, ToryggError> {
        let path = self.wine_user_dir()?.join(String::from("Local Settings/Application Data/") + self.name());
        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::DirectoryNotFound(path))
        }
    }
}

/// appid: Steam app id
/// name: Directory inside "$LIBRARY/steamapps/common" that the app is installed into
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SteamApp {
    pub appid: usize,
    pub name: &'static str
}

impl AsRef<SteamApp> for SteamApp {
    fn as_ref(&self) -> &SteamApp {
        self
    }
}

impl Serialize for SteamApp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for SteamApp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl<S> Game for S where S: AsRef<SteamApp> {
    fn install_dir(&self) -> Result<PathBuf, ToryggError> {
        let path = util::steam_library(self.as_ref())?
            .join("steamapps/common")
            .join(self.as_ref().name);

        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::DirectoryNotFound(path))
        }
    }

    fn wine_pfx(&self) -> Result<PathBuf, ToryggError> {
        let path = util::steam_library(self.as_ref())?
            .join("steamapps/compatdata")
            .join(self.as_ref().appid.to_string())
            .join("pfx");

        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::PrefixNotFound)
        }
    }

    fn name(&self) -> &'static str {
        self.as_ref().name
    }

    fn wine_user_dir(&self) -> Result<PathBuf, ToryggError> {
        // Prioritise a path specified via environment variable
        if let Some(str) = std::env::var_os("TORYGG_USER_DIRECTORY") {
            let path = PathBuf::from(str);
            return if path.exists() {
                Ok(path)
            } else {
                Err(ToryggError::DirectoryNotFound(path))
            }
        }

        let mut path = self.wine_pfx()?;
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
}

pub const SKYRIM: SteamApp = SteamApp {
    appid: 72850,
    name: "Skyrim"
};
pub const SKYRIM_SPECIAL_EDITION: SteamApp = SteamApp {
    appid: 489830,
    name: "Skyrim Special Edition"
};

impl std::str::FromStr for SteamApp {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        Ok(match s {
            s if s == SKYRIM.name() || s == "skyrim" => SKYRIM,
            s if s == SKYRIM_SPECIAL_EDITION.name() || s == "skyrimse" => SKYRIM_SPECIAL_EDITION,
            _ => anyhow::bail!("Unknown game \"{s}\""),
        })
    }
}
