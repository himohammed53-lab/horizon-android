# Mod files on Android (`.hmod`)

Sandboxed `.hmod` mods are WebAssembly components. They are imported in the
launcher's Mods page like any other content file; no new app build is needed
for a mod that only uses functions this build already offers. Executing them
requires a 64-bit build. `.hnmod` source patches are not accepted.

## Building a mod file

The **Build a .hmod mod** workflow (`.github/workflows/android-mod-pilot.yml`)
is started by hand from the Actions tab. It builds the mod folders named in
`mod_dirs` (default: the two examples below) with `cargo horizon-mod build`,
checks them with `cargo horizon-mod check` and uploads the `.hmod` files as the
`hmod` artifact. It does not build the app.

## API level 8

Level 8 adds two functions (`crates/horizon-mod-sdk/wit/horizon.wit`):

- `render.set-brightness(value)`: how bright the world is drawn beyond the
  Brightness option (1 is that option's "Bright", at most 16, 0 gives the
  player's option back). With several mods the brightest counts; the bundled
  Gamma setting counts as one more.
- `world.break-blocks(player, dimension, blocks)` (server mods): breaks blocks
  as the player of this game would. The held tool wears and the drops fall; a
  block the held tool cannot harvest is skipped, and so are the rest once the
  tool has one use left. These
  breaks do not reach `block-broken` again.

A mod that imports them is refused by a build below level 8 with the usual
"update Horizon" message.

## Examples

| Folder | Mod id | Side | What it does |
|---|---|---|---|
| `crates/horizon-mod-sdk/examples/vein-miner` | `vein_miner_file` | server | Mining an ore breaks the connected vein (at most 32 blocks); `/vein on` and `/vein off` |
| `crates/horizon-mod-sdk/examples/gamma` | `gamma_file` | client | `/gamma 1` to `/gamma 16`, `/gamma off`; starts at 8 |

Their ids differ from the bundled `vein_miner` and `gamma` rows so both can be
installed; turn one of each pair off to avoid doing the work twice.

Verified on a device: Gamma. Not yet: `break-blocks`.
