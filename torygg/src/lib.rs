mod config;
mod games;
mod error;
pub use error::ToryggError as Error;
mod profile;
pub use profile::Profile;
mod util;
mod modmanager;
mod state;
mod fomod;
pub use fomod::{
    Plugin,
    FileGroup,
    GroupType,
};

pub use state::ToryggState as Torygg;