use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use log::{error, info};
use simplelog::TermLogger;
use walkdir::WalkDir;

use torygg::{get_installed_mods, get_profiles, get_profile_dir, util::verify_directory, config, games, applauncher::AppLauncher, is_mod_active, Profile, install_mod_from_archive, uninstall_mod, activate_mod, deactivate_mod, create_mod, create_profile};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    /// The game to operate on
    #[arg(long)]
    game: &'static games::SteamApp,

    #[command(subcommand)]
    subcommand: Subcommands
}

#[derive(Subcommand)]
enum Subcommands {
    /// list installed / active mods
    ListMods {
        /// profile to show active mods from
        #[arg(long)]
        profile: Profile,
    },

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
        /// profile to activate the mod on
        #[arg(long)]
        profile: Profile,

        /// name of mod to activate
        #[arg(long)]
        name: String,
    },

    /// deactivate a mod
    Deactivate {
        /// profile to deactivate the mod on
        #[arg(long)]
        profile: Profile,

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

    /// create a new profile
    CreateProfile {
        /// name of the profile to create
        #[arg(long)]
        name: String,
    },

    /// delete a profile
    DeleteProfile {
        /// name of the profile to delete
        #[arg(long)]
        name: Profile,
    },

    /// launch the game with mods
    Run {
        /// profile to run
        #[arg(long)]
        profile: Profile,
    },

    /// view the contents of the overwrite directory
    Overwrite {
        /// profile which to show the overwrite directory of
        #[arg(long)]
        profile: Profile,
    },
}

fn main() {
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

    // Verify directories exist
    if let Err(e) = || -> Result<(), &'static str> {
        verify_directory(&config::get_data_dir()?)?;
        verify_directory(&config::get_mods_dir()?)?;
        verify_directory(&config::get_overwrite_dir()?)?;
        verify_directory(&config::get_profiles_dir()?)?;
        verify_directory(&config::get_data_dir()?.join("Configs"))
    }() {
        error!("{}", e);
        return;
    }

    match &cli.subcommand {
        Subcommands::ListMods { profile } => {
            info!("Listing mods");
            println!("Mods");
            let mods = match get_installed_mods() {
                Ok(mods) => mods.into_iter(),
                Err(e) => {
                    error!("{}", e);
                    return;
                }
            };

            let statuses = mods.clone().map(|m| match is_mod_active(profile.get_name(), &m) {
                Ok(enabled) => {
                    if enabled {
                        "*"
                    } else {
                        ""
                    }
                }
                Err(_) => "",
            });
        
            let combined = mods.zip(statuses);
        
            for m in combined {
                println!("{}{}", m.1, m.0)
            }
            
        },
        Subcommands::Install { archive, name } => {
            info!("Installing {} as {name}", archive.display());
            if let Err(e) = install_mod_from_archive(archive, name) {
                error!("{}", e);
            }
        },

        Subcommands::Uninstall { name } => {
            info!("Uninstalling {name}");
            if let Err(e) = uninstall_mod(name) {
                error!("{}", e);
            }
        },

        Subcommands::Activate { profile, name } => {
            info!("Activating {name}");
            if let Err(e) = activate_mod(profile.get_name(), name) {
                error!("{}", e);
            } 
        },

        Subcommands::Deactivate { profile, name } => {
            info!("Deactivating {name}");
            if let Err(e) = deactivate_mod(profile.get_name(), name) {
                error!("{}", e);
            }
        },

        Subcommands::CreateMod { name } => {
            info!("Creating new mod with name: {name}");
            if let Err(e) = create_mod(name) {
                error!("{}", e);
            }
        },

        Subcommands::ListProfiles => {
            info!("Listing profiles");
            match get_profiles() {
                Ok(profiles) => {
                    for profile in profiles {
                        println!("{}", profile.get_name())
                    }
                },
                Err(e) => error!("{e}")
            }
        },

        Subcommands::CreateProfile { name } => {
            info!("Creating a profile with name: {name}");
            if let Err(e) = create_profile(name) {
                error!("{e}");
            }
        },

        Subcommands::DeleteProfile { name } => {
            info!("Deleting profile with name: {}", name.get_name());
            match || -> anyhow::Result<()> {
                let dir = get_profile_dir(name.get_name()).unwrap();
                fs::remove_dir_all(dir)?;
                Ok(())
            }() {
                Ok(_) => (),
                Err(e) => error!("{e}")
            }
        }

        Subcommands::Overwrite { /*profile*/ .. } => {
            info!("Listing overwrite directory contents");
            for e in WalkDir::new(config::get_overwrite_dir().unwrap()).min_depth(1) {
                println!("{}", e.unwrap().path().display());
            }
        },

        Subcommands::Run { profile } => {
            info!("Running the game");
            let mut launcher = AppLauncher::new(cli.game, profile.get_name());

            if let Err(err) = launcher.run() {
                error!("{}", err);
            }
        },
    }
}
