# torygg-vdf

Tiny little [VDF](https://developer.valvesoftware.com/wiki/KeyValues) parser - if you could call it that.

I created this as steamy-vdf couldn't parse my libraryfolders.vdf. So far I have only implemented enough for to get what I need out of it.  

Currently the API is subject to change, feedback / contributions are welcome.

## Is it fast?

Currently on my system (i5 6600k), with 5 library folders and 33 games, both the libraryfolders and findgame examples take 1-3ms to run.