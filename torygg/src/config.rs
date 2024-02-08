use std::sync::OnceLock;
use crate::existing_directory::ExistingDirectory;

static CONFIG_DIR: OnceLock<ExistingDirectory> = OnceLock::new();
static DATA_DIR: OnceLock<ExistingDirectory> = OnceLock::new();

/// # Panics
///
/// Panics if already initialized
pub fn init(config: ExistingDirectory, data: ExistingDirectory) {
    CONFIG_DIR.set(config).expect("failed to initialize config_dir");
    DATA_DIR.set(data).expect("failed to initialize data_dir");
}

/// # Panics
///
/// Panics if either the config or data directories could not be determined or created
pub fn init_default() {
    let config = ExistingDirectory::maybe_create(dirs::config_dir().expect("could not find location for config directory"))
            .expect("could not create user config directory")
            .maybe_create_child_directory("torygg")
            .expect("could not create config directory");

    let data = ExistingDirectory::maybe_create(dirs::data_dir().expect("could not find location for data directory"))
            .expect("could not create user data directory")
            .maybe_create_child_directory("torygg")
            .expect("could not create data directory");

    init(config, data);
}

/// Get the config directory
///
/// # Panics
///
/// Panics when `CONFIG_DIR` has not been initialized
pub fn config_dir() -> &'static ExistingDirectory {
    CONFIG_DIR.get().expect("config dir not initialized")
}

/// Get the data directory
///
/// # Panics
///
/// Panics when `DATA_DIR` has not been initialized
pub fn data_dir() -> &'static ExistingDirectory {
    DATA_DIR.get().expect("data dir not initialized")
}

/// Get the directory in which torygg stores its mods
///
/// # Panics
///
/// Panics when `DATA_DIR` has not been initialized
pub fn mods_dir() -> ExistingDirectory {
    data_dir().maybe_create_child_directory("Mods").expect("Could not create mods directory")
}
