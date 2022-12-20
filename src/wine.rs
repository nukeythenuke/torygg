use std::collections::HashMap;
use std::path::PathBuf;

pub struct Prefix {
    pub wine_exec: PathBuf,
    pub pfx: PathBuf,
    pub env: HashMap<String, String>,
}

impl Prefix {
    pub fn new(pfx: PathBuf) -> Prefix {
        // TODO: Find the correct wine executable
        let wine_exec = PathBuf::from("/usr/bin/wine");

        // TODO: Find the correct environment variables
        let mut env = HashMap::new();
        env.insert("WINE_PREFIX".to_owned(), pfx.to_string_lossy().to_string());

        Prefix {
            wine_exec,
            pfx,
            env,
        }
    }
}
