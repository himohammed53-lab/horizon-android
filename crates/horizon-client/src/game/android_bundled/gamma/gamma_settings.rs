// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_settings (a hook point): the buttons of Gamma's
// settings page.
pub(super) fn run(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::android_bundled::gamma as g;
    if mod_id != g::ID {
        return;
    }
    let game = &*game;
    let settings = g::settings(|key| game.native_setting(g::ID, key));
    buttons.extend(settings.labels());
}
