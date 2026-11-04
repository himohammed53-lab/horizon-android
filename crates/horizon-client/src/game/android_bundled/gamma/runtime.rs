// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork, not taken from an upstream mod.
// Gamma: the game's brightness option stops at 1 (Bright); this mod lets the
// light of dark places such as caves go beyond that. This file is the mod's
// settings with no dependency on the game, so it is also tested alone
// (tools/test_android_bundled_mods.py). The renderer's side is `gamma_frame.rs`.
use std::cell::RefCell;

/// The mod's id: its settings folder.
pub const ID: &str = "gamma";
pub const NAME: &str = "Gamma";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str =
    "Brighten dark places such as caves beyond the game's brightness limit.";
/// The brightness the settings page cycles through; 1 is the game's own
/// "Bright".
pub const LEVELS: [u8; 5] = [1, 2, 4, 8, 16];
const DEFAULT_LEVEL: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    /// One of `LEVELS`.
    pub level: u8,
}

impl Settings {
    /// From the stored values (`get` reads one key); defaults for the ones
    /// missing or unknown.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let enabled = get("enabled").is_none_or(|v| v.trim() != "false");
        let level = get("level")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .filter(|v| LEVELS.contains(v))
            .unwrap_or(DEFAULT_LEVEL);
        Self { enabled, level }
    }
    /// The settings page's buttons.
    pub fn labels(&self) -> Vec<String> {
        let on = if self.enabled { "ON" } else { "OFF" };
        vec![
            format!("{NAME}: {on}"),
            format!("Brightness: {}x", self.level),
        ]
    }
    /// A click on button `i` of `labels`.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => self.enabled = !self.enabled,
            1 => {
                // An unknown level counts as the first, as Zoom's does.
                let at = LEVELS.iter().position(|&l| l == self.level).unwrap_or(0);
                self.level = LEVELS[(at + 1) % LEVELS.len()];
            }
            _ => {}
        }
    }
    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); 2] {
        let flag = if self.enabled { "true" } else { "false" }.to_owned();
        [("enabled", flag), ("level", self.level.to_string())]
    }
    /// The renderer's brightness for this setting: the game's own scale runs
    /// 0 (Moody) to 1 (Bright), and the light is pushed toward its brightest
    /// form in proportion, so a larger number reaches further into the dark.
    pub fn brightness(&self) -> f32 {
        f32::from(self.level)
    }
}

/// What the renderer's brightness should be: the mod's when it is on, else
/// the player's own option.
pub fn brightness(on: bool, settings: Settings, own: f32) -> f32 {
    if on && settings.enabled {
        settings.brightness()
    } else {
        own
    }
}

#[derive(Default)]
struct State {
    /// Read on first use, then kept.
    settings: Option<Settings>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    STATE.with_borrow_mut(|s| *s.settings.get_or_insert_with(|| Settings::read(get)))
}

/// Replaces the settings (the settings page changed them).
pub fn set_settings(settings: Settings) {
    STATE.with_borrow_mut(|s| s.settings = Some(settings));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
    }

    #[test]
    fn defaults_and_unknown_values() {
        assert_eq!(
            Settings::read(stored(&[])),
            Settings {
                enabled: true,
                level: 8
            }
        );
        let odd = Settings::read(stored(&[("enabled", "false\n"), ("level", "5")]));
        assert_eq!(
            odd,
            Settings {
                enabled: false,
                level: 8
            }
        );
    }

    #[test]
    fn settings_round_trip() {
        let s = Settings {
            enabled: false,
            level: 16,
        };
        let round: Vec<(&str, String)> = s.stored().into();
        let back: Vec<(&str, &str)> = round.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Settings::read(stored(&back)), s);
    }

    #[test]
    fn buttons_toggle_and_cycle() {
        let mut s = Settings {
            enabled: true,
            level: 16,
        };
        s.click(1);
        assert_eq!(s.level, 1, "the cycle wraps");
        s.click(1);
        s.click(0);
        assert_eq!(s.labels(), ["Gamma: OFF", "Brightness: 2x"]);
        s.click(9);
        assert_eq!(
            (s.enabled, s.level),
            (false, 2),
            "an unknown button does nothing"
        );
    }

    #[test]
    fn the_mod_sets_the_brightness_only_while_it_is_on() {
        let s = Settings {
            enabled: true,
            level: 4,
        };
        assert_eq!(brightness(true, s, 0.5), 4.0);
        assert_eq!(
            brightness(false, s, 0.5),
            0.5,
            "turned off in the Mods page"
        );
        let off = Settings {
            enabled: false,
            level: 4,
        };
        assert_eq!(
            brightness(true, off, 0.5),
            0.5,
            "switched off in its own page"
        );
    }

    #[test]
    fn settings_are_kept_until_reset() {
        reset();
        let mut s = settings(stored(&[]));
        s.enabled = false;
        set_settings(s);
        assert!(!settings(stored(&[])).enabled);
        reset();
        assert!(settings(stored(&[])).enabled);
    }
}

/// Reset profile-scoped state when Android returns from its launcher.
pub(super) fn reset() {
    STATE.with_borrow_mut(|state| *state = State::default());
}
