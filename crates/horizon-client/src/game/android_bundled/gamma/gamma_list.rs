// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_mods (a hook point): Gamma's row in Options > Mods.
pub(super) fn run(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::android_bundled::gamma as g;
    mods.push(crate::game::native_hooks::NativeMod {
        id: g::ID.into(),
        title: format!("{} {}", g::NAME, g::VERSION),
        description: g::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
