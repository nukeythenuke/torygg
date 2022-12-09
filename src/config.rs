use crate::util::verify_directory;
use std::path::PathBuf;
use crate::error::ToryggError;

static APP_NAME: &str = "torygg";

static MODS_SUBDIR: &str = "Mods";
static OVERWRITE_SUBDIR: &str = "Overwrite";
static PROFILES_SUBDIR: &str = "Profiles";
static CONFIGS_SUBDIR: &str = "Configs";

pub fn get_data_dir() -> Result<PathBuf, ToryggError> {
    let dir = dirs::data_dir()
        .ok_or(ToryggError::Other("Could not find torygg's data dir".to_owned()))?
        .join(APP_NAME);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_mods_dir() -> Result<PathBuf, ToryggError> {
    let dir = get_data_dir()?.join(MODS_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_profiles_dir() -> Result<PathBuf, ToryggError> {
    let dir = get_data_dir()?.join(PROFILES_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_configs_dir() -> Result<PathBuf, ToryggError> {
    let dir = get_data_dir()?.join(CONFIGS_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}
