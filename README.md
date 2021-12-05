# torygg
A mod manager for Skyrim Special Edition on linux

WIP.

Requires in path:
- [p7zip](https://github.com/jinfeihan57/p7zip) for extracting archives
- [fuse-overlayfs](https://github.com/containers/fuse-overlayfs) to mount mods on top of the Skyrim data directory
- [protontricks](https://github.com/Matoking/protontricks) to launch the game

## Usage
```bash
torygg install <path/to/mod_archive> <desired_mod_name> # Install a mod  
torygg activate <mod_name>  # Activate a mod  
torygg run  # Mount the overlayfs, run the game, then unmount
torygg help  # List commands
```  

## Info

OverlayFS is used to overlay mods on top of the Skyrim data directory, an "overwrite" directory is overlayed at the top that will catch files created and modified when the game is run leaving the data directory unmodified.

Todo:
- Manipulation of the load order.
- FOMODS? For now your best option is to install the mod in another mod manager then copy the folder over.
- Other things that I can't think of.
