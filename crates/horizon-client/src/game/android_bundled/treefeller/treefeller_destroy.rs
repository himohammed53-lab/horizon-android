// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_destroy_block (a hook point): a player breaks a
// block, once the game allowed it. When it is a log of a tree (Falling Trees)
// or a block of an ore (Vein Miner), the connected rest is broken first, one
// block at a time, as that player: the held tool wears for each, the drops
// fall and the game's own rules (adventure mode, protected places) apply.
use crate::game::android_bundled::treefeller as tf;
use crate::game::{Game, KeyCode};

pub(super) fn run(game: &mut Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    if *canceled {
        return;
    }
    // Each break below reaches this hook again: only the player's own starts
    // a search.
    let Some(_busy) = tf::Busy::enter() else {
        return;
    };
    let origin = name_at(game, pos);
    let sneaking = game.keys.contains(&KeyCode::ShiftLeft);
    for kind in tf::Kind::ALL {
        // Off in the Mods page, or switched off in the mod's own page.
        if !game.native_on(kind.id()) {
            continue;
        }
        let settings = tf::settings(kind, |key| game.native_setting(kind.id(), key));
        if !settings.enabled || (settings.sneak_only && !sneaking) {
            continue;
        }
        let limit = usize::from(settings.limit);
        let targets = {
            let view = &*game;
            match kind {
                tf::Kind::Trees if view.items().block_in_tag("minecraft:logs", &origin) => {
                    tf::tree(
                        pos,
                        limit,
                        |p| name_at(view, p) == origin,
                        |p| {
                            view.items()
                                .block_in_tag("minecraft:leaves", &name_at(view, p))
                        },
                    )
                }
                tf::Kind::Veins if is_ore(&origin) && right_tool(view, &origin) => {
                    tf::vein(pos, limit, |p| name_at(view, p) == origin)
                }
                _ => continue,
            }
        };
        break_all(game, kind, &origin, targets);
        return;
    }
}

/// `minecraft:iron_ore`, `minecraft:deepslate_diamond_ore`, ancient debris...
fn is_ore(name: &str) -> bool {
    name.ends_with("_ore") || name == "minecraft:ancient_debris"
}

/// The name of the block at `pos`, `minecraft:air` where nothing is loaded.
fn name_at(game: &Game<'_>, pos: [i32; 3]) -> String {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    game.res.facts().get(state).name.to_string()
}

/// Whether the held tool makes `block` drop: a vein broken with the wrong
/// tool would only be lost. Creative breaks never drop and never wear.
fn right_tool(game: &Game<'_>, block: &str) -> bool {
    game.creative || game.res.items().correct_for_drops(game.held(), block)
}

/// The held tool has one use left: the player's own break may take it, ours
/// may not.
fn tool_spent(game: &Game<'_>) -> bool {
    !game.creative
        && game.held().is_some_and(|s| {
            game.res
                .items()
                .get(&s.item)
                .is_some_and(|i| i.max_damage > 0 && s.damage + 1 >= i.max_damage)
        })
}

fn break_all(game: &mut Game<'_>, kind: tf::Kind, origin: &str, targets: Vec<[i32; 3]>) {
    for at in targets {
        if tool_spent(game) {
            break;
        }
        // An earlier break may have taken it with it.
        if name_at(game, at) != origin {
            continue;
        }
        if kind == tf::Kind::Veins && !right_tool(game, origin) {
            break;
        }
        let _ = game.break_block_as_player(at);
    }
}
