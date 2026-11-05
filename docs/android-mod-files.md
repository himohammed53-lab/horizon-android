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

## API level 9

Level 9 adds `game.get-view()` (`crates/horizon-mod-sdk/wit/horizon.wit`): the
camera the last frame was drawn from (position, yaw, pitch, vertical field of
view). With `render.gui-size` a client mod can project a point of the world to
the screen, for markers such as waypoints. A mod that imports it is refused by
a build below level 9 with the "update Horizon" message.

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

## Mods

| Folder | Mod id | Side | What it does |
|---|---|---|---|
| `crates/horizon-mod-sdk/examples/falling-trees` | `falling_trees_file` | server | Breaking the bottom log of a tree brings the trunk down (at most 128 logs, leaves must touch it); `/tree on` and `/tree off` |
| `crates/horizon-mod-sdk/examples/vein-miner` | `vein_miner_file` | server | Mining an ore breaks the connected vein (at most 32 blocks); `/vein on` and `/vein off` |
| `crates/horizon-mod-sdk/examples/gamma` | `gamma_file` | client | Brighter light; a settings page in Options > Mods (ON/OFF, brightness 1 to 16), `/gamma 1` to `/gamma 16`, `/gamma off`; starts at 8 |
| `crates/horizon-mod-sdk/examples/waypoints` | `waypoints_file` | client | Waypoints drawn on the screen (grow as you near, name, colour, distance); a skull where you last died; list in Options > Mods, `/wp help` in chat. Needs API level 9 |

Server mods have no settings page (pages belong to client mods with capability
`ui`); they are switched with their command. The game's own mods in
`mods/official` (zoom, minimap, recipes, keystrokes, fps-overlay, far-view) are
built by the same workflow: put their folders in `mod_dirs`.

Not yet verified on a device: `break-blocks` and the two server mods.

## Launcher and keys

- The Mods page's switch for the bundled mods reads ON (green) or OFF (red).
- `.hnmod` source mods are not listed: they cannot be installed here.
- A mod's key row in Options > Mods (a key the mod registered, for example
  Gamma's) opens the system keyboard: a letter, digit, Space or Enter binds
  it; closing the keyboard keeps the old key. A key can also be bound to a
  touch button in the touch layout editor.
