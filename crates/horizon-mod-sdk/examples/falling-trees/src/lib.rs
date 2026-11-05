//! Horizon server mod: break the bottom log of a tree and the trunk above it
//! comes down. A tree is a trunk of connected logs that has leaves against it,
//! so a log pile or a house is left alone. The logs are broken highest first
//! as the player (`world.break-blocks`: drops fall and the held tool wears).
use horizon_mod_sdk::server::{
    Args, Argument, BlockPos, Player, Registration, ServerMod, world, world::BlockRef,
};
use std::collections::{HashSet, VecDeque};

/// Logs of one tree, besides the one the player broke.
const LIMIT: usize = 128;
/// Blocks looked at while searching, so a forest cannot stall a tick.
const SCAN: usize = 512;

struct FallingTrees {
    on: bool,
}

/// `minecraft:oak_log[axis=y]` without its properties.
fn block_name(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

fn is_log(name: &str) -> bool {
    ["_log", "_wood", "_stem", "_hyphae"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

fn is_leaves(name: &str) -> bool {
    name.ends_with("_leaves")
}

/// The logs of the tree standing on `origin` (the broken log): joined to it
/// through the 26 neighbours, never lower than it, highest first. Empty when
/// no leaves touch them.
fn tree(dimension: &str, origin: (i32, i32, i32), name: &str) -> Vec<BlockRef> {
    let mut seen = HashSet::from([origin]);
    let mut queue = VecDeque::from([origin]);
    let mut logs: Vec<(i32, i32, i32)> = Vec::new();
    let mut leaves = false;
    let mut looked = 0;
    'search: while let Some((x, y, z)) = queue.pop_front() {
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let at = (x + dx, y + dy, z + dz);
                    if at.1 < origin.1 || !seen.insert(at) {
                        continue;
                    }
                    looked += 1;
                    if looked > SCAN {
                        break 'search;
                    }
                    let Some(state) = world::get_block(dimension, at.0, at.1, at.2) else {
                        continue;
                    };
                    let found = block_name(&state);
                    if is_leaves(found) {
                        leaves = true;
                    } else if found == name {
                        logs.push(at);
                        if logs.len() == LIMIT {
                            break 'search;
                        }
                        queue.push_back(at);
                    }
                }
            }
        }
    }
    if !leaves {
        return Vec::new();
    }
    logs.sort_by_key(|p| std::cmp::Reverse(p.1));
    logs.into_iter()
        .map(|(x, y, z)| BlockRef { x, y, z })
        .collect()
}

impl ServerMod for FallingTrees {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            FallingTrees { on: true },
            Registration::new()
                .command_with("tree", [Argument::word("mode").optional()])
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
            Some(other) => return Err(format!("/tree on or /tree off, not {other}")),
        }
        Ok(Some(format!(
            "Falling Trees is {}",
            if self.on { "on" } else { "off" }
        )))
    }

    fn block_broken(&mut self, player: &Player, pos: BlockPos, state: &str) {
        let name = block_name(state);
        if !self.on || !is_log(name) {
            return;
        }
        let logs = tree(&player.dimension, (pos.x, pos.y, pos.z), name);
        if !logs.is_empty() {
            let _ = world::break_blocks(&player.id, &player.dimension, &logs);
        }
    }
}

horizon_mod_sdk::export_server_mod!(FallingTrees);
