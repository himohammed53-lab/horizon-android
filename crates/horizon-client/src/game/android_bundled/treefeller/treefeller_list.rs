// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_mods (a hook point): Falling Trees' and Vein
// Miner's rows in Options > Mods.
pub(super) fn run(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::android_bundled::treefeller as tf;
    for kind in tf::Kind::ALL {
        mods.push(crate::game::native_hooks::NativeMod {
            id: kind.id().into(),
            title: format!("{} {}", kind.name(), tf::VERSION),
            description: kind.description().into(),
            keys: Vec::new(),
        });
    }
}
