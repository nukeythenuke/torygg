# torygg
A mod manager for Skyrim Special Edition on linux

WIP.

Requires:
- [archivemount](https://github.com/cybernoid/archivemount) (archives are mounted, then a squashfs image is created from the mounted directory)
- [squashfuse](https://github.com/vasi/squashfuse) (to mount squashfs images)
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

Uses SquashFS images to store installed mods, these images are then mounted in temporary directories. OverlayFS is then used to overlay the mods on top of the Skyrim data directory, an "overwrite" directory is overlayed at the top that will catch files created and modified when the game is run leaving the data directory unmodified.

Todo:
- Launch Skyrim not through Steam incase Steam wants to update it which might break mods.
- Now modifies the Plugins.txt, however the implementation is dumb and needs improved.
- Manipulation of the load order.
- FOMODS?
- Other things that I can't think of.

This is pretty much my first rust project so it is likely a mess, any pointers will be much appreciated.
Contributions and thoughts are welcome.
