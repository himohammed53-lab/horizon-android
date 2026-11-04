// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork.
// native_hooks::Game::native_frame (a hook point): once a frame, hand the
// renderer the brightness the mod wants, or the player's own option when the
// mod is off. The renderer takes its brightness from this field when it
// rebuilds the light map, so a value above 1 lights the dark further.
pub(super) fn run(game: &mut crate::game::Game<'_>) {
    use crate::game::android_bundled::gamma as g;
    let own = game.settings.gamma;
    let on = game.native_on(g::ID);
    let settings = g::settings(|key| game.native_setting(g::ID, key));
    let wanted = g::brightness(on, settings, own);
    if let Some(renderer) = game.renderer.as_mut()
        && renderer.gamma != wanted
    {
        renderer.gamma = wanted;
    }
}
