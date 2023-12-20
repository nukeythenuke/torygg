# torygg-cli
Cli frontend for [torygg](../torygg)

Requires in path:
- [p7zip](https://github.com/p7zip-project/p7zip) for extracting archives

## Usage
```bash
torygg-cli install <path/to/mod_archive> [desired_mod_name] # Install a mod  
torygg-cli activate <mod_name> # Activate a mod  
torygg-cli deploy # Copy modded files to the game
# Run loot to generate your load order
# Run the game
torygg-cli help  # List commands
```