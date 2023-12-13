use crate::util::verify_directory;
use std::path::PathBuf;
use std::sync::OnceLock;

static APP_NAME: &str = "torygg";

/// Get torygg's config directory
///
/// # Panics
/// Panics when `dirs::config_dir` returns `None` or the directory does not exist and cannot be created
pub fn config_dir() -> &'static PathBuf {
    static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
    CONFIG_DIR.get_or_init(|| {
        let  path = dirs::config_dir().expect("could not find location for config directory");
        let dir = path.join(APP_NAME);
        verify_directory(&dir).expect("could not create config directory");
        dir
    })
}

/// Get torygg's data directory
///
/// # Panics
/// Panics when `dirs::data_dir()` returns `None` or the directory does not exist and cannot be created
pub fn data_dir() -> &'static PathBuf {
    static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
    DATA_DIR.get_or_init(|| {
        let path = dirs::data_dir().expect("Could not find location for data directory");
        let dir = path.join(APP_NAME);
        verify_directory(&dir).expect("Could not create data directory");
        dir
    })
}

/// Get the directory in which torygg stores its mods for a given game
///
/// # Panics
/// Panics when `data_dir` panics or the directory does not exist and cannot be created
pub fn mods_dir() -> &'static PathBuf {
    static MODS_DIR: OnceLock<PathBuf> = OnceLock::new();
    MODS_DIR.get_or_init(|| {
        let dir = data_dir().join("mods");
        verify_directory(&dir).expect("Could not create mods directory");
        dir
    })
}
