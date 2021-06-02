# torygg
A mod manager for Skyrim Special Edition on linux

WIP.

Uses SquashFS images to store installed mods, these images are then mounted on temporary directories. OverlayFS is then used to overlay the mods on top of the Skyrim data directory, an "overwrite" directory is overlayed last that will contain files created and modified when the game is run.
