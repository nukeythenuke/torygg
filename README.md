# torygg
A mod manager for Skyrim Special Edition on linux

WIP.

Requires:
- [archivemount](https://github.com/cybernoid/archivemount) (archives are mounted, then a squashfs image is created from the mounted directory)
- [squashfuse](https://github.com/vasi/squashfuse) (to mount squashfs images)
- [fuse-overlayfs](https://github.com/containers/fuse-overlayfs) (to mount mods on top of the Skyrim data directory)

## Usage
```bash
export TORYGG_SKYRIM_INSTALL_DIRECTORY=/path/to/Skyrim/folder  # Needed to find the Skyrim folder
export TORYGG_USER_DIRECTORY=/path/to/wine/user/dir  # Needed to find wine/proton user dir for to handle configs and plugins.txt

torygg install <path/to/mod_archive> <desired_mod_name> # Install a mod  
torygg activate <mod_name>  # Activate a mod  
torygg mount  # Mount the mod overlayfs over the skyrim data directory
```  
Now launch Skyrim through Steam.
Skyrim (I think) will detect all plugins and fill the plugins.txt.
You will then need to then add a `*` beside each entry in the plugins.txt file and modify the load order to suit.  
```bash
torygg umount  # Unmount the overlayfs  
torygg help  # List of commands
```

## Info

Uses SquashFS images to store installed mods, these images are then mounted in temporary directories. OverlayFS is then used to overlay the mods on top of the Skyrim data directory, an "overwrite" directory is overlayed at the top that will catch files created and modified when the game is run leaving the data directory unmodified.

Todo:
- Currently doesn't modify the plugins.txt, so that has to be done manually.
- Manipulation of the load order
- Other things that I can't think of

This is pretty much my first rust project so it is likely a mess, any pointers will be much appreciated.
Contributions and thoughts are welcome.
