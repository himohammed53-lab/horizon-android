# Fixed Android mod sources

The parent `android_bundled.rs` wires a fixed, opt-in set of MIT native modules
into existing game hook points. It is not a native-mod loader or compiler.
`PROVENANCE.json` pins source revisions; every module retains its MIT LICENSE.

The runtime and adapter sources come from the corresponding upstream mod.
Helper files are renamed and expose `run` for static calls. Original
`horizon-mixin.toml` files are retained for reference, not used for building:
this directory is not a directly installable `.hnmod` bundle.

`treefeller` and `gamma` are the exception: original code of this fork, not upstream mods.
It adds Falling Trees (break the bottom log and the trunk above it comes down)
and Vein Miner (mine one ore and the connected vein breaks), each a row in
Options > Mods with its own settings. Both run from the `native_destroy_block`
hook and break the extra blocks through the game's own player break.
Gamma lets the renderer's brightness go past the game's own limit, so caves
are lit; it sets the value each frame and hands the player's own option back
when it is off. All three are listed in the Mods page (`ANDROID_BUNDLED` in
`mod_list.rs`) and, like the other bundled mods, start off until enabled there.
The Minimap that was bundled here has been removed from this fork.

Android changes reset profile-scoped state on launcher transitions, retain the
Android debug text scale and preserve the code-of-conduct screen. Update the
portable runtime tests and Android checks when changing these adapters.
