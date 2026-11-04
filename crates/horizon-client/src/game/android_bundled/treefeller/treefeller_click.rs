// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_settings_click (a hook point): a click on Falling
// Trees' or Vein Miner's settings page changes the setting and keeps it.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::android_bundled::treefeller as tf;
    let Some(kind) = tf::Kind::from_id(mod_id) else {
        return;
    };
    let mut settings = tf::settings(kind, |key| game.native_setting(kind.id(), key));
    settings.click(button);
    tf::set_settings(settings);
    for (key, value) in settings.stored() {
        game.native_set_setting(kind.id(), key, &value);
    }
}
