use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use log::info;
use simplelog::TermLogger;
use walkdir::WalkDir;

use torygg::{games, applauncher::AppLauncher, profile::{Profile, get_profiles}, modmanager};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    /// The game to operate on
    #[arg(long)]
    game: games::SteamApp,

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
            let Some(mods) = profile.get_enabled_mods() else {
                println!("No mods.");
                return Ok(());
            };

            println!("Mods");
            for m in mods {
                println!("{}{}", if profile.is_mod_enabled(m) { "*" } else { "" }, m)
            }
            
        },
        Subcommands::Install { profile, archive, name } => {
            info!("Installing {} as {name}", archive.display());
            modmanager::install_mod(profile.get_game(), archive, name)?
        },

        Subcommands::Uninstall { profile, name } => {
            info!("Uninstalling {name}");
            modmanager::uninstall_mod(profile.get_game(), name)?
        },

        Subcommands::Activate { profile, name } => {
            info!("Activating {name}");
            let mut profile = profile.clone();
            profile.enable_mod(name)
        },

        Subcommands::Deactivate { profile, name } => {
            info!("Deactivating {name}");
            let mut profile = profile.clone();
            profile.disable_mod(name);
        },

        Subcommands::CreateMod { profile, name } => {
            info!("Creating new mod with name: {name}");
            modmanager::create_mod(profile.get_game(), name)?;
        },

        Subcommands::ListProfiles => {
            info!("Listing profiles");
            for profile in get_profiles()? {
                println!("{}", profile.get_name())
            }
        },

        Subcommands::CreateProfile { name } => {
            info!("Creating a profile with name: {name}");
            Profile::new(name, cli.game)?;
        },

        Subcommands::DeleteProfile { profile } => {
            info!("Deleting profile with name: {}", profile.get_name());
            let dir = profile.get_dir()?;
            fs::remove_dir_all(dir)?;
        }

        Subcommands::Overwrite { profile } => {
            info!("Listing overwrite directory contents");
            for e in WalkDir::new(profile.get_overwrite_dir()?).min_depth(1) {
                println!("{}", e?.path().display());
            }
        },

        Subcommands::Run { profile } => {
            info!("Running the game");
            let mut launcher = AppLauncher::new(profile);

            launcher.run()?;
        },
    }

    Ok(())
}
