use std::fs;
use std::path::PathBuf;
use anyhow::anyhow;

use clap::{Parser, Subcommand};
use log::{error, info};
use simplelog::TermLogger;
use walkdir::WalkDir;

use torygg::{
    get_profiles,
    util::verify_directory,
    config,
    games,
    applauncher::AppLauncher,
    Profile};

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
        /// profile to install the mod into
        #[arg(long)]
        profile: Profile,

        /// mod archive to install
        #[arg(long)]
        archive: PathBuf,

        /// the name of the installed mod
        #[arg(long)]
        name: String,
    },

    /// uninstall a mod
    Uninstall {
        /// profile to uninstall the mod from
        profile: Profile,

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
        /// profile to create the mod in
        profile: Profile,

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
        /// profile to delete
        #[arg(long)]
        profile: Profile,
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

    match &cli.subcommand {
        Subcommands::ListMods { profile } => {
            info!("Listing mods");
            println!("Mods");
            for (m, enabled) in profile.get_mods() {
                println!("{}{}", if *enabled { "*" } else { "" }, m)
            }
            
        },
        Subcommands::Install { profile, archive, name } => {
            info!("Installing {} as {name}", archive.display());
            if let Err(e) = profile.install_mod(archive, name) {
                error!("{}", e);
            }
        },

        Subcommands::Uninstall { profile, name } => {
            info!("Uninstalling {name}");
            if let Err(e) = profile.uninstall_mod(name) {
                error!("{}", e);
            }
        },

        Subcommands::Activate { profile, name } => {
            info!("Activating {name}");
            let mut profile = profile.clone();
            if let Err(e) = profile.enable_mod(name) {
                error!("{}", e);
            } 
        },

        Subcommands::Deactivate { profile, name } => {
            info!("Deactivating {name}");
            let mut profile = profile.clone();
            if let Err(e) = profile.disable_mod(name) {
                error!("{}", e);
            }
        },

        Subcommands::CreateMod { profile, name } => {
            info!("Creating new mod with name: {name}");
            if let Err(e) = profile.create_mod(name) {
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
            if let Err(e) = Profile::new(name) {
                error!("{e}");
            }
        },

        Subcommands::DeleteProfile { profile } => {
            info!("Deleting profile with name: {}", profile.get_name());
            let dir = profile.get_dir().map_err(|e| anyhow!(e))?;
            fs::remove_dir_all(dir)?;
        }

        Subcommands::Overwrite { /*profile*/ .. } => {
            info!("Listing overwrite directory contents");
            for e in WalkDir::new(config::get_overwrite_dir().unwrap()).min_depth(1) {
                println!("{}", e.unwrap().path().display());
            }
        },

        Subcommands::Run { profile } => {
            info!("Running the game");
            let mut launcher = AppLauncher::new(cli.game, profile);

            if let Err(err) = launcher.run() {
                error!("{}", err);
            }
        },
    }

    Ok(())
}
