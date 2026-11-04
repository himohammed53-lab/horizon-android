// Copyright (c) 2026 the Horizon Android contributors
// SPDX-License-Identifier: MIT; see LICENSE in this directory.
// Original module of this Android fork, not taken from an upstream mod.
// Falling Trees and Vein Miner: breaking one log of a tree, or one block of an
// ore vein, breaks the rest that is connected to it. This file is the mods'
// settings and the search for what to break, with no dependency on the game,
// so it is also tested alone (tools/test_android_bundled_mods.py). The break
// itself is `treefeller_destroy.rs`.
use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};

pub const VERSION: &str = "0.1.0";

/// The two mods that share this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Trees,
    Veins,
}

impl Kind {
    pub const ALL: [Kind; 2] = [Kind::Trees, Kind::Veins];

    /// The mod's id: its settings folder.
    pub const fn id(self) -> &'static str {
        match self {
            Kind::Trees => "falling_trees",
            Kind::Veins => "vein_miner",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Kind::Trees => "Falling Trees",
            Kind::Veins => "Vein Miner",
        }
    }
    pub const fn description(self) -> &'static str {
        match self {
            Kind::Trees => {
                "Break the bottom log of a tree and the whole trunk above it comes down."
            }
            Kind::Veins => {
                "Mine one block of an ore vein and the whole connected vein breaks with it."
            }
        }
    }
    pub fn from_id(id: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.id() == id)
    }
    /// The maximum number of blocks one break may take, one per click.
    const fn limits(self) -> [u16; 4] {
        match self {
            Kind::Trees => [32, 64, 128, 256],
            Kind::Veins => [16, 32, 64, 128],
        }
    }
    const fn default_limit(self) -> u16 {
        match self {
            Kind::Trees => 128,
            Kind::Veins => 32,
        }
    }
    /// A vein is easy to trigger by accident while digging, so it waits for
    /// the player to sneak; a tree does not.
    const fn default_sneak_only(self) -> bool {
        matches!(self, Kind::Veins)
    }
}

/// One mod's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub kind: Kind,
    pub enabled: bool,
    /// Act only while the player sneaks.
    pub sneak_only: bool,
    /// The most blocks one break may take, the broken one included.
    pub limit: u16,
}

impl Settings {
    /// From the stored values (`get` reads one key); defaults for the ones
    /// missing or unknown.
    pub fn read(kind: Kind, get: impl Fn(&str) -> Option<String>) -> Self {
        let flag = |key: &str, default: bool| match get(key).as_deref().map(str::trim) {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        };
        let limit = get("limit")
            .and_then(|v| v.trim().parse::<u16>().ok())
            .filter(|v| kind.limits().contains(v))
            .unwrap_or(kind.default_limit());
        Self {
            kind,
            enabled: flag("enabled", true),
            sneak_only: flag("sneak", kind.default_sneak_only()),
            limit,
        }
    }
    /// The settings page's buttons.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("{}: {}", self.kind.name(), on(self.enabled)),
            format!("Only while sneaking: {}", on(self.sneak_only)),
            format!("Most blocks at once: {}", self.limit),
        ]
    }
    /// A click on button `i` of `labels`.
    pub fn click(&mut self, i: usize) {
        match i {
            0 => self.enabled = !self.enabled,
            1 => self.sneak_only = !self.sneak_only,
            2 => {
                let limits = self.kind.limits();
                let at = limits.iter().position(|&l| l == self.limit).unwrap_or(0);
                self.limit = limits[(at + 1) % limits.len()];
            }
            _ => {}
        }
    }
    /// What to store, key by key.
    pub fn stored(&self) -> [(&'static str, String); 3] {
        let flag = |b: bool| if b { "true" } else { "false" }.to_owned();
        [
            ("enabled", flag(self.enabled)),
            ("sneak", flag(self.sneak_only)),
            ("limit", self.limit.to_string()),
        ]
    }
}

#[derive(Default)]
struct State {
    /// Read on first use, then kept; one per `Kind::ALL`.
    settings: [Option<Settings>; 2],
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

fn slot(kind: Kind) -> usize {
    match kind {
        Kind::Trees => 0,
        Kind::Veins => 1,
    }
}

/// The settings of `kind`, read once through `get`.
pub fn settings(kind: Kind, get: impl Fn(&str) -> Option<String>) -> Settings {
    STATE.with_borrow_mut(|s| {
        *s.settings[slot(kind)].get_or_insert_with(|| Settings::read(kind, get))
    })
}

/// Replaces the settings (the settings page changed them).
pub fn set_settings(settings: Settings) {
    STATE.with_borrow_mut(|s| s.settings[slot(settings.kind)] = Some(settings));
}

/// Held while this module breaks blocks: each of those breaks reaches the
/// same hook again, which must not start another search.
pub struct Busy(());

impl Busy {
    pub fn enter() -> Option<Busy> {
        BUSY.with(|b| (!b.replace(true)).then_some(Busy(())))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        BUSY.with(|b| b.set(false));
    }
}

/// The 26 positions around `p`, edges and corners included: a branch may
/// leave a trunk diagonally, and an ore vein often touches only at a corner.
fn around(p: [i32; 3]) -> [[i32; 3]; 26] {
    let mut out = [[0; 3]; 26];
    let mut n = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if (dx, dy, dz) != (0, 0, 0) {
                    out[n] = [p[0] + dx, p[1] + dy, p[2] + dz];
                    n += 1;
                }
            }
        }
    }
    out
}

/// The blocks connected to `origin` that `accept`, nearest first, without
/// `origin` itself; with it, at most `limit` blocks.
pub fn flood(origin: [i32; 3], limit: usize, accept: impl Fn([i32; 3]) -> bool) -> Vec<[i32; 3]> {
    let mut seen = HashSet::from([origin]);
    let mut queue = VecDeque::from([origin]);
    let mut found = Vec::new();
    while let Some(at) = queue.pop_front() {
        for next in around(at) {
            if found.len() + 1 >= limit {
                return found;
            }
            if seen.insert(next) && accept(next) {
                found.push(next);
                queue.push_back(next);
            }
        }
    }
    found
}

/// How many logs the search for a tree's leaves looks through.
const SCAN: usize = 512;

/// The logs to break with the log at `origin`, highest first: the logs
/// connected to it that are no lower than it, so that cutting the bottom
/// brings the trunk down and cutting higher leaves the stump alone. Empty when
/// no leaves touch the logs: that is a pole or a wall someone built, not a
/// tree.
pub fn tree(
    origin: [i32; 3],
    limit: usize,
    is_log: impl Fn([i32; 3]) -> bool,
    is_leaves: impl Fn([i32; 3]) -> bool,
) -> Vec<[i32; 3]> {
    // The leaves are at the top, so the search for them reaches further than
    // the limit on what is broken: a tall tree is still a tree when the limit
    // takes only its lower logs.
    let mut logs = flood(origin, SCAN.max(limit), |p| p[1] >= origin[1] && is_log(p));
    let grown = std::iter::once(origin)
        .chain(logs.iter().copied())
        .any(|p| around(p).into_iter().any(&is_leaves));
    if !grown || limit < 2 {
        return Vec::new();
    }
    logs.truncate(limit - 1);
    // Stable: logs of one height stay nearest first.
    logs.sort_by_key(|p| std::cmp::Reverse(p[1]));
    logs
}

/// The blocks of a vein to break with the one at `origin`, nearest first.
pub fn vein(origin: [i32; 3], limit: usize, same: impl Fn([i32; 3]) -> bool) -> Vec<[i32; 3]> {
    flood(origin, limit, same)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
    }

    /// A world of named blocks; everything else is air.
    struct World(HashMap<[i32; 3], &'static str>);

    impl World {
        fn new(blocks: &[([i32; 3], &'static str)]) -> Self {
            Self(blocks.iter().copied().collect())
        }
        fn is(&self, name: &'static str) -> impl Fn([i32; 3]) -> bool + '_ {
            move |p| self.0.get(&p) == Some(&name)
        }
    }

    /// An oak: a trunk of `height` logs at x=0,z=0 from y=0, leaves on top.
    fn oak(height: i32) -> Vec<([i32; 3], &'static str)> {
        let mut blocks: Vec<_> = (0..height).map(|y| ([0, y, 0], "log")).collect();
        blocks.push(([0, height, 0], "leaves"));
        blocks
    }

    #[test]
    fn cutting_the_bottom_log_takes_the_trunk_highest_first() {
        let world = World::new(&oak(5));
        let found = tree([0, 0, 0], 100, world.is("log"), world.is("leaves"));
        assert_eq!(found, vec![[0, 4, 0], [0, 3, 0], [0, 2, 0], [0, 1, 0]]);
    }

    #[test]
    fn cutting_higher_leaves_the_stump_below() {
        let world = World::new(&oak(5));
        let found = tree([0, 2, 0], 100, world.is("log"), world.is("leaves"));
        assert_eq!(found, vec![[0, 4, 0], [0, 3, 0]]);
    }

    #[test]
    fn a_pole_without_leaves_is_not_a_tree() {
        let blocks: Vec<_> = (0..6).map(|y| ([0, y, 0], "log")).collect();
        let world = World::new(&blocks);
        assert!(tree([0, 0, 0], 100, world.is("log"), world.is("leaves")).is_empty());
    }

    #[test]
    fn diagonal_branches_come_down_and_other_wood_does_not() {
        let mut blocks = oak(3);
        blocks.extend([
            ([1, 2, 1], "log"),
            ([2, 3, 2], "log"),
            ([2, 4, 2], "leaves"),
        ]);
        blocks.push(([0, 1, 1], "birch_log"));
        let world = World::new(&blocks);
        let mut found = tree([0, 0, 0], 100, world.is("log"), world.is("leaves"));
        found.sort();
        assert_eq!(found, vec![[0, 1, 0], [0, 2, 0], [1, 2, 1], [2, 3, 2]]);
    }

    #[test]
    fn a_neighbouring_tree_out_of_reach_stays() {
        let mut blocks = oak(4);
        blocks.extend((0..4).map(|y| ([3, y, 0], "log")));
        blocks.push(([3, 4, 0], "leaves"));
        let world = World::new(&blocks);
        let found = tree([0, 0, 0], 100, world.is("log"), world.is("leaves"));
        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|p| p[0] == 0));
    }

    #[test]
    fn the_limit_counts_the_broken_block_and_keeps_the_nearest() {
        let world = World::new(&oak(10));
        let found = tree([0, 0, 0], 4, world.is("log"), world.is("leaves"));
        assert_eq!(found, vec![[0, 3, 0], [0, 2, 0], [0, 1, 0]]);
        assert!(tree([0, 0, 0], 1, world.is("log"), world.is("leaves")).is_empty());
        assert!(tree([0, 0, 0], 0, world.is("log"), world.is("leaves")).is_empty());
    }

    #[test]
    fn a_vein_follows_corners_and_stops_at_other_blocks() {
        let world = World::new(&[
            ([0, 0, 0], "iron"),
            ([1, 1, 1], "iron"),
            ([2, 1, 1], "iron"),
            ([0, 0, 1], "stone"),
            ([5, 0, 0], "iron"),
        ]);
        let mut found = vein([0, 0, 0], 50, world.is("iron"));
        found.sort();
        assert_eq!(found, vec![[1, 1, 1], [2, 1, 1]]);
        assert_eq!(vein([0, 0, 0], 2, world.is("iron")), vec![[1, 1, 1]]);
    }

    #[test]
    fn settings_default_read_and_round_trip() {
        let trees = Settings::read(Kind::Trees, stored(&[]));
        assert_eq!(
            trees,
            Settings {
                kind: Kind::Trees,
                enabled: true,
                sneak_only: false,
                limit: 128
            }
        );
        let veins = Settings::read(Kind::Veins, stored(&[]));
        assert_eq!(
            veins,
            Settings {
                kind: Kind::Veins,
                enabled: true,
                sneak_only: true,
                limit: 32
            }
        );
        let odd = Settings::read(
            Kind::Veins,
            stored(&[("enabled", "false\n"), ("sneak", "false"), ("limit", "999")]),
        );
        assert_eq!(
            odd,
            Settings {
                kind: Kind::Veins,
                enabled: false,
                sneak_only: false,
                limit: 32
            }
        );
        let round: Vec<(&str, String)> = odd.stored().into();
        let back: Vec<(&str, &str)> = round.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Settings::read(Kind::Veins, stored(&back)), odd);
    }

    #[test]
    fn buttons_toggle_and_cycle() {
        let mut s = Settings::read(Kind::Trees, stored(&[]));
        s.click(0);
        s.click(1);
        s.click(2);
        assert_eq!(
            s.labels(),
            [
                "Falling Trees: OFF",
                "Only while sneaking: ON",
                "Most blocks at once: 256"
            ]
        );
        s.click(2);
        assert_eq!(s.limit, 32, "the cycle wraps");
        s.click(9);
        assert_eq!(s.limit, 32, "an unknown button does nothing");
    }

    #[test]
    fn kinds_are_found_by_id() {
        for kind in Kind::ALL {
            assert_eq!(Kind::from_id(kind.id()), Some(kind));
        }
        assert_eq!(Kind::from_id("zoom"), None);
    }

    #[test]
    fn settings_are_kept_per_mod_until_reset() {
        reset();
        let mut trees = settings(Kind::Trees, stored(&[]));
        trees.enabled = false;
        set_settings(trees);
        assert!(!settings(Kind::Trees, stored(&[])).enabled);
        assert!(settings(Kind::Veins, stored(&[])).enabled);
        reset();
        assert!(settings(Kind::Trees, stored(&[])).enabled);
    }

    #[test]
    fn only_one_search_runs_at_a_time() {
        let first = Busy::enter().expect("free");
        assert!(
            Busy::enter().is_none(),
            "the breaks it makes come back here"
        );
        drop(first);
        assert!(Busy::enter().is_some());
    }
}

/// Reset profile-scoped state when Android returns from its launcher.
pub(super) fn reset() {
    STATE.with_borrow_mut(|state| *state = State::default());
}
