# Scythe native mod (draft)

Black scythe with chain targeting on P, a crit that cuts mobs in half, and sound effects.

Status: not compiled and not run. Hook and event field names are assumed from the Horizon docs and must be checked with `native.py check` and a build.

## Layout
- `engine/src/lib.rs`: typed trait API (chain targeting, crit cut, sounds)
- `mixins/scythe/`: injection bundle (P key cooldown, tick)

## Known gaps
- Backhand grip animation: not implemented. `render_hand_pose` only moves the first-person hand.
- Chain visual line: not implemented.
- Crit cut and sound names (`CriticalHitEvent` fields, `kill_mob`, `play_sound` argument order, vanilla sound IDs) are unverified.
- Non-crit slash: not implemented.

## Build
1. `python3 native/tools/native.py check mods/scythe`
2. Build the game with the launcher's native build step.
