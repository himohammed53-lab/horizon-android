// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Source revision: e3ee0a7c450b28c3ed58fa59a4881ec78c5ee2e0. Android static-bundle adaptation.
// The zoom key went down or up: the player's choice of key, kept as the
// WebAssembly Zoom kept it.
pub(super) fn run(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::official_zoom as zoom;
    // Turned off in Options > Mods: nothing until the next build leaves it out.
    if !game.native_on(zoom::ID) {
        return;
    }
    let bound = game.native_mod_key(zoom::ID, zoom::KEY, zoom::DEFAULT_KEY);
    if key != bound {
        // The zoom in/out keys, only on press and only while the zoom is held.
        if pressed {
            for (name, default, zoom_in) in [
                (zoom::KEY_IN, zoom::DEFAULT_IN, true),
                (zoom::KEY_OUT, zoom::DEFAULT_OUT, false),
            ] {
                if key == game.native_mod_key(zoom::ID, name, default) {
                    if !zoom::active() {
                        let playing = game.native_playing();
                        let g = &*game;
                        zoom::latch(playing, |n| g.native_setting(zoom::ID, n));
                    }
                    zoom::step(zoom_in);
                }
            }
        }
        return;
    }
    let playing = game.native_playing();
    let game = &*game;
    zoom::key(playing, pressed, |name| game.native_setting(zoom::ID, name));
}
