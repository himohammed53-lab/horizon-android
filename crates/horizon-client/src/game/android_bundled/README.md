# Fixed Android mod sources

The parent `android_bundled.rs` wires a fixed, opt-in set of MIT native modules
into existing game hook points. It is not a native-mod loader or compiler.
`PROVENANCE.json` pins source revisions; every module retains its MIT LICENSE.

The runtime and adapter sources come from the corresponding upstream mod.
Helper files are renamed and expose `run` for static calls. Original
`horizon-mixin.toml` files are retained for reference, not used for building:
this directory is not a directly installable `.hnmod` bundle.

Recipes and SmartF3 were removed from this fork to keep the game lean; Zoom is the one
bundled mod left.

Falling Trees, Vein Miner and Gamma are not bundled here: they are `.hmod`
mod files (`crates/horizon-mod-sdk/examples`, docs/android-mod-files.md). The
one piece of code they need in the game is `mod_brightness.rs`, which hands
the renderer the brightness a mod file asks for each frame.

Android changes reset profile-scoped state on launcher transitions, retain the
Android debug text scale and preserve the code-of-conduct screen. Update the
portable runtime tests and Android checks when changing these adapters.
