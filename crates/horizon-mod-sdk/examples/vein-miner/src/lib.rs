//! Horizon server mod: mining an ore breaks the whole connected vein.
//!
//! It listens for broken blocks (`block-broken`), floods the connected blocks
//! of the same ore, and asks the game to break them as the player
//! (`world.break-blocks`: drops fall, the held tool wears, and a block the
//! tool cannot harvest is left alone).
use horizon_mod_sdk::server::{
    Args, Argument, BlockPos, Player, Registration, ServerMod, world, world::BlockRef,
};
use std::collections::HashSet;

/// Blocks of one vein, besides the one the player broke.
const LIMIT: usize = 32;

struct VeinMiner {
    on: bool,
}

/// `minecraft:iron_ore[...]` without its properties.
fn block_name(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

fn is_ore(name: &str) -> bool {
    name.ends_with("_ore") || name == "minecraft:ancient_debris"
}

/// The blocks joined to `origin` (26 neighbours) that are `name`, nearest
/// first, at most [`LIMIT`].
fn vein(dimension: &str, origin: (i32, i32, i32), name: &str) -> Vec<BlockRef> {
    let mut seen = HashSet::from([origin]);
    let mut queue = std::collections::VecDeque::from([origin]);
    let mut found = Vec::new();
    // A scan bound too, so a vein of thousands cannot stall the tick.
    let mut looked = 0;
    while let Some((x, y, z)) = queue.pop_front() {
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let at = (x + dx, y + dy, z + dz);
                    if !seen.insert(at) {
                        continue;
                    }
                    looked += 1;
                    if looked > 512 {
                        return found;
                    }
                    let same = world::get_block(dimension, at.0, at.1, at.2)
                        .is_some_and(|s| block_name(&s) == name);
                    if !same {
                        continue;
                    }
                    found.push(BlockRef {
                        x: at.0,
                        y: at.1,
                        z: at.2,
                    });
                    if found.len() == LIMIT {
                        return found;
                    }
                    queue.push_back(at);
                }
            }
        }
    }
    found
}

impl ServerMod for VeinMiner {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            VeinMiner { on: true },
            Registration::new()
                .command_with("vein", [Argument::word("mode").optional()])
                .on_blocks(),
        ))
    }

    fn command(
        &mut self,
        _player: &Player,
        _name: &str,
        args: &Args,
    ) -> Result<Option<String>, String> {
        match args.string("mode") {
            Some("on") => self.on = true,
            Some("off") => self.on = false,
            None => {}
            Some(other) => return Err(format!("/vein on or /vein off, not {other}")),
        }
        Ok(Some(format!(
            "Vein Miner is {}",
            if self.on { "on" } else { "off" }
        )))
    }

    fn block_broken(&mut self, player: &Player, pos: BlockPos, state: &str) {
        let name = block_name(state);
        if !self.on || !is_ore(name) {
            return;
        }
        let blocks = vein(&player.dimension, (pos.x, pos.y, pos.z), name);
        if !blocks.is_empty() {
            let _ = world::break_blocks(&player.id, &player.dimension, &blocks);
        }
    }
}

horizon_mod_sdk::export_server_mod!(VeinMiner);
