use crate::util::verify_directory;
use std::path::PathBuf;

static APP_NAME: &str = "torygg";

static MODS_SUBDIR: &str = "Mods";
static OVERWRITE_SUBDIR: &str = "Overwrite";
static PROFILES_SUBDIR: &str = "Profiles";

pub fn get_data_dir() -> Result<PathBuf, &'static str> {
    let dir = dirs::data_dir()
        .ok_or("Could not find torygg's data dir")?
        .join(APP_NAME);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_mods_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(MODS_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_overwrite_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(OVERWRITE_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}

pub fn get_profiles_dir() -> Result<PathBuf, &'static str> {
    let dir = get_data_dir()?.join(PROFILES_SUBDIR);
    verify_directory(&dir)?;
    Ok(dir)
}
