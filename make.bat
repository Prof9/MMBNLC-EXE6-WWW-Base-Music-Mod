@echo off
cargo build --release
copy "target\release\patch.dll" "C:\Program Files (x86)\Steam\steamapps\common\MegaMan_BattleNetwork_LegacyCollection_Vol2\exe\mods\FixWWWBaseMusic_EXE6"
