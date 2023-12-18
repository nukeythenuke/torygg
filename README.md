# torygg
A mod manager for Skyrim Special Edition on linux

WIP.

Requires in path:
- [p7zip](https://github.com/p7zip-project/p7zip) for extracting archives

## Features
- Install / Uninstall mods
- FOMOD support (largely untested)
- Profiles

## Usage
```bash
torygg-cli install <path/to/mod_archive> [desired_mod_name] # Install a mod  
torygg-cli activate <mod_name> # Activate a mod  
torygg-cli deploy # Copy modded files to the game
# Run loot to generate your load order
# Run the game
torygg-cli help  # List commands
```

## Todo
- Manipulation of the load order (loose files).
- Other things that I can't think of.
