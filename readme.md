MMBNLC Postgame WWW Base Music mod
==================================

This is a mod for Mega Man Battle Network Legacy Collection Vol. 2 adjusts the
field music played in the WWW base in Mega Man Battle Network 6 after the game
is completed.

Normally, after beating the game, the original Expo Site music will play in that
area again from before it was revealed to be the WWW base. This mod makes it so
that the WWW base music will continue to play in that area. All other areas will
have their normal area music.


Features
--------

* After beating the game, WWW base music is kept as-is rather than reverting to
  the original Expo Site theme.


Installing
----------

Windows PC and Steam Deck

1. Download and install chaudloader: https://github.com/RockmanEXEZone/chaudloader/releases Version 0.8.1 or newer is required.

2. Launch Steam in Desktop Mode. Right-click the game in Steam, then click Properties → Local Files → Browse to open the game's install folder. Then open the "exe" folder, where you'll find MMBN_LC2.exe.

3. Copy the PostgameWWWBaseMusic_EXE6 folder to the "mods" folder.

4. Launch the game as normal.


Version History
---------------

Ver. 1.0.1 - 15 October 2023

* Updated to work with Steam version 1.0.0.3.

Ver. 1.0.0 - 8 May 2023

* Initial version.


Building
--------

Building is supported on Windows 10 & 11. First install the following prerequisites:

* Rust

Then, run one of the following commands:

* make - Builds the mod files compatible with chaudloader.
* make clean - Removes all temporary files and build outputs.
* make install - Installs the previously built mod files into the mods folder for chaudloader.
* make uninstall - Removes the installed mod files from the mods folder for chaudloader.
