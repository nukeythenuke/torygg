use std::collections::HashMap;
use crate::util;
use std::path::PathBuf;
use crate::error::ToryggError;

pub struct SteamApp {
    appid: usize,
    name: &'static str
}

impl SteamApp {
    #[must_use]
    pub fn appid(&self) -> usize {
        self.appid
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The games installation directory
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    pub fn install_dir(&self) -> Result<PathBuf, ToryggError> {
        let path = util::steam_library(self)?
            .join("steamapps/common")
            .join(self.name);

        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::DirectoryNotFound(path))
        }
    }

    /// The wine prefix associated with the game
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    pub fn wine_pfx(&self) -> Result<PathBuf, ToryggError> {
        let path = util::steam_library(self)?
            .join("steamapps/compatdata")
            .join(self.appid.to_string())
            .join("pfx");

        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::PrefixNotFound)
        }
    }

    /// The windows user directory in the wine prefix
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    pub fn wine_user_dir(&self) -> Result<PathBuf, ToryggError> {
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

    /// The games config directory
    ///
    /// # Errors
    /// Errors when the directory cannot be found
    pub fn config_dir(&self) -> Result<PathBuf, ToryggError> {
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
    pub fn appdata_dir(&self) -> Result<PathBuf, ToryggError> {
        let path = self.wine_user_dir()?.join(String::from("Local Settings/Application Data/") + self.name());
        if path.exists() {
            Ok(path)
        } else {
            Err(ToryggError::DirectoryNotFound(path))
        }
    }
}

// pub const SKYRIM: SteamApp = SteamApp {
//     appid: 72850,
//     name: "Skyrim"
// };

pub const SKYRIM_SPECIAL_EDITION: SteamApp = SteamApp {
    appid: 489830,
    name: "Skyrim Special Edition"
};

