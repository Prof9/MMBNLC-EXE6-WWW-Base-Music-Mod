@echo off
cargo build
copy "target\debug\patch.dll" "C:\Program Files (x86)\Steam\steamapps\common\MegaMan_BattleNetwork_LegacyCollection_Vol2\exe\mods\FixWWWBaseMusic_EXE6"