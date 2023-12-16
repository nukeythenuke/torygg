use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use log::info;
use serde::{Deserialize, Serialize};
use simplelog::TermLogger;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

use torygg::{profile::{Profile, profiles}, modmanager};
use torygg::applauncher::AppLauncher;
use torygg::config::{data_dir};
use torygg::error::ToryggError;

mod serde_profile {
    use std::fmt::Formatter;
    use std::str::FromStr;
    use serde::{de, Deserializer, Serializer};
    use serde::de::{Visitor};
    use torygg::profile::{Profile};

    struct ProfileVisitor;

    impl<'de> Visitor<'de> for ProfileVisitor {
        type Value = Profile;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            write!(formatter, "name of a profile")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: de::Error {
            Profile::from_str(v).map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
        }
    }

    pub fn serialize<S>(profile: &Profile, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer  {
        serializer.serialize_str(profile.name())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Profile, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_str(ProfileVisitor)
    }
}

/// Torygg's persistent state
#[derive(Serialize, Deserialize)]
struct ToryggState {
    //game: &'static SteamApp,
    #[serde(with = "serde_profile")]
    profile: Profile
}

impl ToryggState {
    // fn game(&self) -> &'static SteamApp {
    //     self.game
    // }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn set_profile(&mut self, name: &str) -> Result<(), ToryggError> {
        self.profile = Profile::from_str(name).map_err(|_| ToryggError::Other("failed to find profile".to_owned()))?;
        Ok(())
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    subcommand: Subcommands
}

#[derive(Subcommand)]
enum Subcommands {
    /// list installed / active mods
    ListMods,

    /// install a mod from an archive
    Install {
        /// mod archive to install
        #[arg(long)]
        archive: PathBuf,

        /// the name of the installed mod
        #[arg(long)]
        name: String,
    },

    /// uninstall a mod
    Uninstall {
        /// name of mod to uninstall
        #[arg(long)]
        name: String,
    },

    /// activate a mod
    Activate {
        /// name of mod to activate
        #[arg(long)]
        name: String,
    },

    /// deactivate a mod
    Deactivate {
        /// name of mod to deactivate
        #[arg(long)]
        name: String,
    },

    /// create a new, empty, mod
    CreateMod {
        /// name of mod to create
        #[arg(long)]
        name: String,
    },

    ListProfiles,

    SetProfile {
      name: String
    },

    /// create a new profile
    CreateProfile {
        /// name of the profile to create
        #[arg(long)]
        name: String,
    },

    /// delete a profile
    DeleteProfile {
        /// profile to delete
        #[arg(long)]
        profile: Profile,
    },

    /// view the contents of the overwrite directory
    Overwrite,

    /// mount the modded directories, use ctrl-c to unmount
    Mount,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    TermLogger::init(
        if cli.verbose {
            simplelog::LevelFilter::Info
        } else {
            simplelog::LevelFilter::Warn
        },
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    {
        let profiles = profiles().unwrap();
        if profiles.is_empty() {
            Profile::new("default").unwrap();
        }
    }

    let state_path = data_dir().join(".toryggstate.toml");

    let mut state = {
        if let Ok(s) = fs::read_to_string(&state_path) {
            toml::from_str(&s).unwrap()
        } else {
            ToryggState { profile: profiles().unwrap().first().unwrap().clone() }
        }
    };

    match &cli.subcommand {
        Subcommands::ListMods => {
            info!("Listing mods");
            let mods = modmanager::installed_mods()?;
            if mods.is_empty() {
                println!("No mods.");
                return Ok(());
            };

            println!("Mods");
            for m in mods {
                println!("{}{}", if state.profile().mod_enabled(&m) { "*" } else { "" }, m);
            }
            
        },
        Subcommands::Install { archive, name } => {
            info!("Installing {} as {name}", archive.display());
            modmanager::install_mod(archive, name)?;
        },

        Subcommands::Uninstall { name } => {
            info!("Uninstalling {name}");
            modmanager::uninstall_mod(name)?;
        },

        Subcommands::Activate { name } => {
            info!("Activating {name}");
            let mut profile = state.profile().clone();
            profile.enable_mod(name);
        },

        Subcommands::Deactivate { name } => {
            info!("Deactivating {name}");
            let mut profile = state.profile().clone();
            profile.disable_mod(name);
        },

        Subcommands::CreateMod { name } => {
            info!("Creating new mod with name: {name}");
            modmanager::create_mod(name)?;
        },

        Subcommands::ListProfiles => {
            info!("Listing profiles");
            let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
            for profile in profiles()? {
                if profile.name() == state.profile().name() {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
                }

                writeln!(&mut stdout, "{}", profile.name()).unwrap();
                stdout.reset().unwrap();
            }
        },

        Subcommands::SetProfile { name } => {
            info!("Setting profile");
            if state.set_profile(name).is_err() {
                println!("failed to set profile: {name}");
            }

            fs::write(&state_path, toml::to_string(&state).unwrap()).unwrap();
        }

        Subcommands::CreateProfile { name } => {
            info!("Creating a profile with name: {name}");
            Profile::new(name)?;
        },

        Subcommands::DeleteProfile { profile } => {
            info!("Deleting profile with name: {}", profile.name());
            let dir = profile.dir()?;
            fs::remove_dir_all(dir)?;
        }

        Subcommands::Overwrite => {
            info!("Listing overwrite directory contents");
            for e in WalkDir::new(state.profile().overwrite_dir()?).min_depth(1) {
                println!("{}", e?.path().display());
            }
        },

        Subcommands::Mount => {
            info!("Mounting modded directories");
            let mut launcher = AppLauncher::new(state.profile());
            launcher.mount_all()?;
        },
    }

    Ok(())
}
