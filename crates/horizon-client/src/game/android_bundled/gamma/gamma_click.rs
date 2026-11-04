// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_settings_click (a hook point): a click on Gamma's
// settings page changes the setting and keeps it.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::android_bundled::gamma as g;
    if mod_id != g::ID {
        return;
    }
    let mut settings = g::settings(|key| game.native_setting(g::ID, key));
    settings.click(button);
    g::set_settings(settings);
    for (key, value) in settings.stored() {
        game.native_set_setting(g::ID, key, &value);
    }
}
