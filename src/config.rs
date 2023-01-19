use crate::util::verify_directory;
use std::path::PathBuf;
use once_cell::sync::OnceCell;

static APP_NAME: &str = "torygg";

pub fn get_config_dir() -> &'static PathBuf {
    static CONFIG_DIR: OnceCell<PathBuf> = OnceCell::new();
    CONFIG_DIR.get_or_init(|| {
        let  path = dirs::config_dir().expect("could not find location for config directory");
        let dir = path.join(APP_NAME);
        verify_directory(&dir).expect("could not create config directory");
        dir
    })
}

pub fn get_data_dir() -> &'static PathBuf {
    static DATA_DIR: OnceCell<PathBuf> = OnceCell::new();
    DATA_DIR.get_or_init(|| {
        let path = dirs::data_dir().expect("Could not find location for data directory");
        let dir = path.join(APP_NAME);
        verify_directory(&dir).expect("Could not create data directory");
        dir
    })
}

pub fn get_mods_dir() -> &'static PathBuf {
    static MODS_DIR: OnceCell<PathBuf> = OnceCell::new();
    MODS_DIR.get_or_init(|| {
        let dir = get_data_dir().join("Mods");
        verify_directory(&dir).expect("Could not create mods directory");
        dir
    })
}

pub fn get_profiles_dir() -> &'static PathBuf {
    static PROFILES_DIR: OnceCell<PathBuf> = OnceCell::new();
    PROFILES_DIR.get_or_init(|| {
        let dir = get_data_dir().join("Profiles");
        verify_directory(&dir).expect("Could not create profiles directory");
        dir
    })
}

pub fn get_configs_dir() -> &'static PathBuf {
    static CONFIGS_DIR: OnceCell<PathBuf> = OnceCell::new();
    CONFIGS_DIR.get_or_init(|| {
        let dir = get_data_dir().join("Configs");
        verify_directory(&dir).expect("Could not create configs directory");
        dir
    })
}
