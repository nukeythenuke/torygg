# torygg
A mod manager for Skyrim Special Edition on linux

WIP. Currently does not work as a change in fuse-overlayfs makes mounting a lowerdir on itself cause processes accessing the mounted directories to hang indefinitely (this was always undefined behaviour).

Requires:
- [p7zip](https://github.com/jinfeihan57/p7zip) for extracting archives
- [fuse-overlayfs](https://github.com/containers/fuse-overlayfs) (to mount mods on top of the Skyrim data directory)

## Usage
```bash
torygg install <path/to/mod_archive> <desired_mod_name> # Install a mod  
torygg activate <mod_name>  # Activate a mod  
torygg mount  # Mount the mod overlayfs over the skyrim data directory
```  
Now launch Skyrim as you normally would through Steam.

Some other commands:
```bash
torygg umount  # Unmount the overlayfs  
torygg help  # List of commands
```

## Info

OverlayFS is used to overlay mods on top of the Skyrim data directory, an "overwrite" directory is overlayed at the top that will catch files created and modified when the game is run leaving the data directory unmodified.

Todo:
- Launch Skyrim not through Steam incase Steam wants to update it which might break mods.
- Now modifies the Plugins.txt, however the implementation is dumb and needs improved.
- Manipulation of the load order.
- FOMODS?
- Other things that I can't think of.

This is pretty much my first rust project so it is likely a mess, any pointers will be much appreciated.
Contributions and thoughts are welcome.
