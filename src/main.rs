use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use log::info;
use simplelog::TermLogger;
use walkdir::WalkDir;

use torygg::{profile::{Profile, profiles}, modmanager};
use torygg::applauncher::AppLauncher;

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
        /// profile to delete
        #[arg(long)]
        profile: Profile,
    },

    /// view the contents of the overwrite directory
    Overwrite {
        /// profile which to show the overwrite directory of
        #[arg(long)]
        profile: Profile,
    },

    /// mount the modded directories, use ctrl-c to unmount
    Mount {
        /// profile to mount
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
            let mods = modmanager::installed_mods()?;
            if mods.is_empty() {
                println!("No mods.");
                return Ok(());
            };

            println!("Mods");
            for m in mods {
                println!("{}{}", if profile.mod_enabled(&m) { "*" } else { "" }, m);
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

        Subcommands::Activate { profile, name } => {
            info!("Activating {name}");
            let mut profile = profile.clone();
            profile.enable_mod(name);
        },

        Subcommands::Deactivate { profile, name } => {
            info!("Deactivating {name}");
            let mut profile = profile.clone();
            profile.disable_mod(name);
        },

        Subcommands::CreateMod { name } => {
            info!("Creating new mod with name: {name}");
            modmanager::create_mod(name)?;
        },

        Subcommands::ListProfiles => {
            info!("Listing profiles");
            for profile in profiles()? {
                println!("{}", profile.name());
            }
        },

        Subcommands::CreateProfile { name } => {
            info!("Creating a profile with name: {name}");
            Profile::new(name)?;
        },

        Subcommands::DeleteProfile { profile } => {
            info!("Deleting profile with name: {}", profile.name());
            let dir = profile.dir()?;
            fs::remove_dir_all(dir)?;
        }

        Subcommands::Overwrite { profile } => {
            info!("Listing overwrite directory contents");
            for e in WalkDir::new(profile.overwrite_dir()?).min_depth(1) {
                println!("{}", e?.path().display());
            }
        },

        Subcommands::Mount {profile } => {
            info!("Mounting modded directories");
            let mut launcher = AppLauncher::new(profile);
            launcher.mount_all()?;
        },
    }

    Ok(())
}
