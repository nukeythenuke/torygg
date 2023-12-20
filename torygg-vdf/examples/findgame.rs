/// Finds the libraryfolder that a game is in
/// Takes the wanted game's appid as its first and only arg

use std::{fs::File, path::PathBuf};

fn main() -> Result<(), &'static str> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        return Err("no app id provided");
    } else if args.len() > 2 {
        return Err("too many args");
    }

    let appid = &args[1];
    println!("{}", appid);

    let vdf = PathBuf::from(std::env::var("HOME").map_err(|_| "failed to find home dir")?)
        .join(".steam/root/config/libraryfolders.vdf");
    let kvs =
        torygg_vdf::parse(&mut File::open(vdf).map_err(|_| "failed to open libraryfolders.vdf")?)
            .map_err(|_| "failed to parse libraryfolders.vdf")?;

    for kv in &kvs {
        let components = kv.0.iter().collect::<Vec<_>>();
        // Key we want:                    🠗
        // libraryfolders/<lib_id>/apps/<appid>
        if let Some(component) = components.get(3) {
            if *component == appid.as_str() {
                // libraryfolders/<lib_id>/path
                let path = PathBuf::from_iter(kv.0.iter().take(2)).join("path");

                println!("{}", &kvs[&path]);
                return Ok(());
            }
        }
    }

    Err("game not found")
}
