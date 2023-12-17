use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToryggError {
    #[error("mod already exists")]
    ModAlreadyExists,

    #[error("profile already exists")]
    ProfileAlreadyExists,

    #[error("torygg is currently deployed")]
    IsDeployed,

    #[error("torygg is not currently deployed")]
    IsNotDeployed,

    #[error("failed to spawn child")]
    FailedToSpawnChild,

    #[error("child failed")]
    ChildFailed,

    #[error("steam library could not be found")]
    SteamLibraryNotFound,

    #[error("wine prefix could not be found")]
    PrefixNotFound,

    #[error("the path is not a directory")]
    NotADirectory(PathBuf),

    #[error("the directory \"{0:?}\" could not found")]
    DirectoryNotFound(PathBuf),

    #[error("IO Error")]
    IOError(#[from] io::Error),

    #[error("{0}")]
    Other(String),

    #[error("unknown error")]
    Unknown
}