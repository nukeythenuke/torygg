use std::fmt::{Debug, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use crate::error::ToryggError;

#[derive(Debug, Clone)]
pub(crate) struct ExistingDirectory {
    path: PathBuf
}

impl ExistingDirectory {
    pub fn maybe_create(path: PathBuf) -> Result<Self, ToryggError> {
        if !path.exists() {
            fs::create_dir(&path).map_err(ToryggError::from)?;
        }

        Self::try_from(path)
    }

    pub fn existing_child_directory<P: AsRef<Path>>(&self, path: P) -> Result<Self, ToryggError> {
        Self::try_from(self.path.join(path))
    }
    pub fn maybe_create_child_directory<P: AsRef<Path>>(&self, path: P) -> Result<Self, ToryggError> {
        Self::maybe_create(self.path.join(path))
    }
}

impl Display for ExistingDirectory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.path.fmt(f)
    }
}

impl TryFrom<PathBuf> for ExistingDirectory {
    type Error = ToryggError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if path.exists() {
            if !path.is_dir() {
                return Err(ToryggError::NotADirectory(path));
            }

            return Ok(Self { path });
        }

        Err(ToryggError::DirectoryNotFound(path))
    }
}

impl AsRef<Path> for ExistingDirectory {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl From<ExistingDirectory> for PathBuf {
    fn from(ed: ExistingDirectory) -> Self {
        ed.path
    }
}

