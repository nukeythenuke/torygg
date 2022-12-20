pub mod applauncher;
pub mod config;
pub mod games;
pub mod error;
pub mod profile;
pub mod util;

pub mod wine {
    use std::collections::HashMap;
    use std::path::PathBuf;

    pub struct Prefix {
        pub wine_exec: PathBuf,
        pub pfx: PathBuf,
        pub env: HashMap<String, String>,
    }

    impl Prefix {
        pub fn new(pfx: PathBuf) -> Prefix {
            Prefix {
                // TODO: Find the correct wine executable
                wine_exec: Default::default(),
                pfx,
                // TODO: Find the correct environment variables
                env: Default::default(),
            }
        }
    }
}