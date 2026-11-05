// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT
// Original module of this Android fork.
// native_hooks::Game::native_frame (a hook point): once a frame, hand the
// renderer the brightness a mod file asks for (`render.set-brightness`), or
// the player's own option when none does. The renderer takes its brightness
// from this field when it rebuilds the light map, so a value above 1 lights
// the dark further.
pub(super) fn run(game: &mut crate::game::Game<'_>) {
    let wanted = game.settings.gamma;
    #[cfg(feature = "mods")]
    let wanted = game.mods.mods.brightness().unwrap_or(wanted);
    if let Some(renderer) = game.renderer.as_mut()
        && renderer.gamma != wanted
    {
        renderer.gamma = wanted;
    }
}
