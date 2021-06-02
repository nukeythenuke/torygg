# torygg
A mod manager for Skyrim Special Edition on linux

WIP.

Uses SquashFS images to store installed mods, these images are then mounted in temporary directories. OverlayFS is then used to overlay the mods on top of the Skyrim data directory, an "overwrite" directory is overlayed at the top that will catch files created and modified when the game is run leaving the data directory unmodified.

Todo:
- Currently doesn't modify the plugins.txt, so that has to be done manually.
- Manipulation of the load order
- Other things that I can't think of

This is pretty much my first rust project so it is likely a mess, any pointers will be much appreciated.
Contributions and thoughts are welcome.
