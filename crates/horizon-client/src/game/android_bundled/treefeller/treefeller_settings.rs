// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_settings (a hook point): the buttons of Falling
// Trees' and Vein Miner's settings pages.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::android_bundled::treefeller as tf;
    let Some(kind) = tf::Kind::from_id(mod_id) else {
        return;
    };
    let game = &*game;
    let settings = tf::settings(kind, |key| game.native_setting(kind.id(), key));
    buttons.extend(settings.labels());
}
