use std::sync::OnceLock;
use crate::existing_directory::ExistingDirectory;

static APP_NAME: &str = "torygg";

/// Get torygg's config directory
///
/// # Panics
/// Panics when `dirs::config_dir` returns `None` or the directory does not exist and cannot be created
pub fn config_dir() -> &'static ExistingDirectory {
    static CONFIG_DIR: OnceLock<ExistingDirectory> = OnceLock::new();
    CONFIG_DIR.get_or_init(|| {
        let  path = dirs::config_dir().expect("could not find location for config directory");
        let dir = path.join(APP_NAME);
        ExistingDirectory::maybe_create(dir).expect("could not create config directory")
    })
}

/// Get torygg's data directory
///
/// # Panics
/// Panics when `dirs::data_dir()` returns `None` or the directory does not exist and cannot be created
pub fn data_dir() -> &'static ExistingDirectory {
    static DATA_DIR: OnceLock<ExistingDirectory> = OnceLock::new();
    DATA_DIR.get_or_init(|| {
        let path = dirs::data_dir().expect("Could not find location for data directory");
        let dir = path.join(APP_NAME);
        ExistingDirectory::maybe_create(dir).expect("Could not create data directory")
    })
}

/// Get the directory in which torygg stores its mods for a given game
///
/// # Panics
/// Panics when `data_dir` panics or the directory does not exist and cannot be created
pub fn mods_dir() -> &'static ExistingDirectory {
    static MODS_DIR: OnceLock<ExistingDirectory> = OnceLock::new();
    MODS_DIR.get_or_init(|| { data_dir().maybe_create_child_directory("Mods").expect("Could not create mods directory") })
}
