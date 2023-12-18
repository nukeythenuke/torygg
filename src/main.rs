use std::collections::HashMap;
use std::io::{stdin, Write};
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use log::info;
use simplelog::TermLogger;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use torygg::{Plugin, Torygg, GroupType, FileGroup};
use torygg::Profile;

fn list_profiles(state: &Torygg) -> Result<(), torygg::Error> {
    let mut stdout = StandardStream::stdout(termcolor::ColorChoice::Always);
    for profile in Torygg::profiles()? {
        if profile.name() == state.profile().name() {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
        }

        writeln!(&mut stdout, "{}", profile.name()).unwrap();
        stdout.reset().unwrap();
    }

    Ok(())
}

fn list_mods(state: &Torygg) -> Result<(), torygg::Error> {
    let mods = Torygg::mods()?;
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
        if state.mod_active(m) {
            stdout.set_color(&active_color).unwrap();
        } else {
            stdout.set_color(&inactive_color).unwrap();
        }

        println!("{m}");
    }

    Ok(())
}

fn print_load_order(state: &Torygg) {
    if let Some(mods) = state.active_mods() {
        for (i, m) in mods.iter().enumerate() {
            println!("{}. {m}", i + 1);
        }
    } else {
        println!("No mods");
    }
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

fn get_input(group: &FileGroup) -> Vec<&Plugin> {
    let options = group.plugins().iter().map(|plugin| {
        (plugin.name(), plugin) }).collect::<HashMap<_, _>>();

    let items = options.keys().collect::<Vec<_>>();
    match group.group_type() {
        GroupType::SelectExactlyOne => {
            let selection = dialoguer::Select::new().with_prompt(group.name())
                .items(&items)
                .interact().unwrap();

            vec![options[items[selection]]]
        }
        GroupType::SelectAny => {
            let selection = dialoguer::MultiSelect::new().with_prompt(group.name())
                .items(&options.keys().collect::<Vec<_>>())
                .interact().unwrap();

            selection.into_iter().map(|i| options[items[i]]).collect()
        }
        GroupType::SelectAll => {
            dialoguer::Select::new().with_prompt(group.name())
                .items(&items).interact().unwrap();

            options.into_values().collect()
        }
    }
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
      profile: Profile
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

    let mut state = Torygg::read_or_new();

    match cli.subcommand {
        Some(Subcommands::ListMods) => list_mods(&state)?,
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
            Torygg::install_mod(&archive, &name, |step| -> Vec<&Plugin> {
                let Some(groups) = step.file_groups() else {
                    return Vec::new()
                };

                println!("{}", step.name());
                groups.iter().flat_map(get_input).collect()
            })?;
        },

        Some(Subcommands::Uninstall { name }) => Torygg::uninstall_mod(&name)?,
        Some(Subcommands::Activate { name }) => state.activate_mod(&name)?,
        Some(Subcommands::Deactivate { name }) => state.deactivate_mod(&name)?,
        Some(Subcommands::CreateMod { name }) => Torygg::create_mod(&name)?,
        Some(Subcommands::LoadOrder) => print_load_order(&state),
        Some(Subcommands::ListProfiles) => list_profiles(&state)?,
        Some(Subcommands::SetProfile { profile }) => state.set_profile(profile)?,
        Some(Subcommands::CreateProfile { name }) => { let _ = Torygg::create_profile(&name)?; },
        Some(Subcommands::DeleteProfile { profile }) => state.delete_profile(&profile)?,
        Some(Subcommands::Deploy) => state.deploy()?,
        Some(Subcommands::Undeploy) => state.undeploy()?,
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
