use std::fs;
use std::io::{stdin, Write};
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use log::{error, info};
use serde::{Deserialize, Serialize};
use simplelog::TermLogger;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

use torygg::{profile::{Profile, profiles}, modmanager};
use torygg::config::{data_dir};
use torygg::error::ToryggError;
use torygg::games::SKYRIM_SPECIAL_EDITION;

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
    profile: Profile,
    deployed: Option<Vec<PathBuf>>
}

impl ToryggState {
    // fn game(&self) -> &'static SteamApp {
    //     self.game
    // }

    fn new() -> ToryggState {
        let state = ToryggState {
            profile: profiles().unwrap().first().unwrap().clone(),
            deployed: None
        };
        state.write().unwrap();
        state
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn profile_mut(&mut self) -> &mut Profile {
        &mut self.profile
    }

    fn set_profile(&mut self, name: &str) -> Result<(), ToryggError> {
        self.profile = Profile::from_str(name).map_err(|_| ToryggError::Other("failed to find profile".to_owned()))?;
        self.write()?;
        Ok(())
    }

    fn path() -> PathBuf {
        data_dir().join(".toryggstate.toml")
    }

    fn read() -> Result<ToryggState, ToryggError> {
        let s = fs::read_to_string(Self::path())?;
        toml::from_str(&s).map_err(|_| ToryggError::Other("Failed to parse state toml".to_owned()))
    }

    fn write(&self) -> Result<(), std::io::Error> {
        fs::write(Self::path(), toml::to_string(self).unwrap())
    }

    fn read_or_new() -> ToryggState {
        ToryggState::read().unwrap_or_else(|_| ToryggState::new())
    }
}

fn list_profiles(state: &ToryggState) -> Result<(), ToryggError> {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
    for profile in profiles()? {
        if profile.name() == state.profile().name() {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
        }

        writeln!(&mut stdout, "{}", profile.name()).unwrap();
        stdout.reset().unwrap();
    }

    Ok(())
}

fn list_mods(state: &ToryggState) -> Result<(), ToryggError> {
    let mods = modmanager::installed_mods()?;
    if mods.is_empty() {
        println!("No mods.");
        return Ok(());
    };

    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
    let mut active_color = ColorSpec::new();
    active_color.set_fg(Some(Color::Green));

    let mut inactive_color = ColorSpec::new();
    inactive_color.set_fg(Some(Color::Red));

    for m in &mods {
        if state.profile().mod_enabled(m) {
            stdout.set_color(&active_color).unwrap();
        } else {
            stdout.set_color(&inactive_color).unwrap();
        }

        println!("{m}");
    }

    Ok(())
}

fn print_header(header: &str) {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);

    let mut header_spec = ColorSpec::new();
    header_spec.set_bold(true);
    header_spec.set_underline(true);

    stdout.set_color(&header_spec).unwrap();
    println!("{header}:");
    stdout.reset().unwrap();
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    subcommand: Option<Subcommands>
}

#[derive(Subcommand)]
enum Subcommands {
    /// list installed / active mods
    ListMods,

    /// install a mod from an archive
    Install {
        /// mod archive to install
        archive: PathBuf,

        /// the name of the installed mod
        name: Option<String>,
    },

    /// uninstall a mod
    Uninstall {
        /// name of mod to uninstall
        name: String,
    },

    /// activate a mod
    Activate {
        /// name of mod to activate
        name: String,
    },

    /// deactivate a mod
    Deactivate {
        /// name of mod to deactivate
        name: String,
    },

    /// create a new, empty, mod
    CreateMod {
        /// name of mod to create
        #[arg(long)]
        name: String,
    },

    LoadOrder,

    ListProfiles,

    SetProfile {
      name: String
    },

    /// create a new profile
    CreateProfile {
        /// name of the profile to create
        name: String,
    },

    /// delete a profile
    DeleteProfile {
        /// profile to delete
        profile: Profile,
    },

    Deploy,

    Undeploy,
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

    let mut state = ToryggState::read_or_new();

    match cli.subcommand {
        Some(Subcommands::ListMods) => {
            list_mods(&state)?;
        },
        Some(Subcommands::Install { archive, name }) => {
            let name = name.unwrap_or_else(|| {
                let default_name = archive.file_stem().unwrap().to_string_lossy().to_string();
                println!("Name for mod: (default: {default_name})");
                let mut name = String::new();
                stdin().read_line(&mut name).unwrap();
                let name = name.trim().to_owned();

                if name.is_empty() {
                    default_name
                } else {
                    name
                }
            });

            info!("Installing {} as {name}", archive.display());
            modmanager::install_mod(&archive, &name)?;
        },

        Some(Subcommands::Uninstall { name }) => {
            info!("Uninstalling {name}");
            modmanager::uninstall_mod(&name)?;
        },

        Some(Subcommands::Activate { name }) => {
            info!("Activating {name}");
            state.profile_mut().enable_mod(&name)?;
        },

        Some(Subcommands::Deactivate { name }) => {
            info!("Deactivating {name}");
            state.profile_mut().disable_mod(&name)?;
        },

        Some(Subcommands::CreateMod { name }) => {
            info!("Creating new mod with name: {name}");
            modmanager::create_mod(&name)?;
        },

        Some(Subcommands::LoadOrder) => {
            print_header("Files");
            if let Some(mods) = state.profile.enabled_mods() {
                for (i, m) in mods.iter().enumerate() {
                    println!("{}. {m}", i + 1);
                }
            } else {
                println!("No mods");
            }
        }

        Some(Subcommands::ListProfiles) => {
            list_profiles(&state)?;
        },

        Some(Subcommands::SetProfile { name }) => {
            info!("Setting profile");
            if state.set_profile(&name).is_err() {
                println!("failed to set profile: {name}");
            }
        }

        Some(Subcommands::CreateProfile { name }) => {
            info!("Creating a profile with name: {name}");
            Profile::new(&name)?;
        },

        Some(Subcommands::DeleteProfile { profile }) => {
            info!("Deleting profile with name: {}", profile.name());
            let dir = profile.dir()?;
            fs::remove_dir_all(dir)?;

            if profile == state.profile {
                ToryggState::new();
            }
        }

        Some(Subcommands::Deploy) => {
            if state.deployed.is_some() {
                error!("Already Deployed")
            } else {
                state.deployed = state.profile.deploy()?;
                state.write().unwrap();
            }

        }

        Some(Subcommands::Undeploy) => {
            if let Some(deployed) = state.deployed {
                for relative_path in deployed.iter().rev() {
                    let path = SKYRIM_SPECIAL_EDITION.install_dir().unwrap().join("Data").join(relative_path);
                    if path.is_dir() {
                        fs::remove_dir(path).unwrap();
                    } else {
                        fs::remove_file(path).unwrap();
                    }
                }

                state.deployed = None;
                state.write().unwrap();

                let backup_dir = data_dir().join("Backup");
                for entry in WalkDir::new(&backup_dir).min_depth(1).contents_first(true) {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    let relative_path = path.strip_prefix(&backup_dir).unwrap();
                    let to_path = SKYRIM_SPECIAL_EDITION.install_dir().unwrap().join("Data").join(relative_path);

                    if path.is_file() {
                        fs::rename(path, to_path).unwrap();
                    } else {
                        fs::remove_dir(path).unwrap();
                    }
                }
            }
        }

        None => {
            print_header("Profiles");
            list_profiles(&state)?;
            println!();
            print_header("Mods");
            list_mods(&state)?;
        }
    }

    Ok(())
}
