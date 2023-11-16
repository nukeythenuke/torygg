use crate::util::verify_directory;
use std::path::PathBuf;
use std::sync::OnceLock;
use crate::games;

static APP_NAME: &str = "torygg";

pub fn config_dir() -> &'static PathBuf {
    static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
    CONFIG_DIR.get_or_init(|| {
        let  path = dirs::config_dir().expect("could not find location for config directory");
        let dir = path.join(APP_NAME);
        verify_directory(&dir).expect("could not create config directory");
        dir
    })
}

pub fn data_dir() -> &'static PathBuf {
    static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
    DATA_DIR.get_or_init(|| {
        let path = dirs::data_dir().expect("Could not find location for data directory");
        let dir = path.join(APP_NAME);
        verify_directory(&dir).expect("Could not create data directory");
        dir
    })
}

pub fn mods_dir(game: &impl games::Game) -> &'static PathBuf {
    static MODS_DIR: OnceLock<PathBuf> = OnceLock::new();
    MODS_DIR.get_or_init(|| {
        let dir = data_dir().join(game.name()).join("mods");
        verify_directory(&dir).expect("Could not create mods directory");
        dir
    })
}
