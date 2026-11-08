// Gravestones for Horizon.
//
// Ported from Gravestones 1.4.2 by Pneumono (MIT), https://modrinth.com/project/Heh3BbSv
// (source: https://github.com/PneumonoIsNotAvailable/Gravestones). The rules here follow that
// mod's `GravestonePlacement`, `GravestoneCreation`, `GravestoneCollection`, `GravestoneDecay`
// and its inventory and experience data types; the blocks, models and textures are the mod's own
// (crates/horizon-mod-sdk/examples/gravestones, with its licence and credits).
//
// When a player dies the grave takes their items and experience, at the nearest good block
// around where they died; a right click on it hands everything back, each stack to the slot it
// came from. It does nothing when the "Gravestones" mod is off in Options > Mods, or when the
// gravestones content mod is not installed (the grave block does not exist then).

use super::Game;
use super::native_block_entity_draw::BlockEntityDraw;
use horizon_assets::item_model::DisplayContext;
use horizon_client::{
    containers::{ModBlockEntity, ModContainer},
    entities::Experience,
    items::{ItemStack, blocks::Fluid},
    text::Text,
};
use horizon_core::random::{LegacyRandom, RandomSource};
use horizon_world::nbt::{Compound, JavaString, Tag};

pub(super) const ID: &str = "gravestones_logic";
pub(super) const VERSION: &str = "1.4.2";
/// The block that holds a player's things (the mod's `gravestone_technical`).
const BLOCK: &str = "gravestones:gravestone_technical";
/// Its block entity.
const ENTITY: &str = "gravestones:technical_gravestone";
/// Slots 0..36 the inventory, 36..40 the armour feet to head, 40 the off hand.
const SLOTS: usize = 41;
/// `GravestonePlacement.RADIUS`.
const RADIUS: i32 = 2;
/// The `damage_to_break` setting's default.
const DAMAGE_TO_BREAK: i32 = 3;
/// The `decay_time` setting's default: 8 hours, in ticks.
const DECAY_TICKS: i64 = 8 * 60 * 60 * 20;
/// `experience_cap`: at most this many points are kept.
const EXPERIENCE_CAP: i32 = 100;

/// The mod's row in Options > Mods.
pub(super) fn list(mods: &mut Vec<super::native_hooks::NativeMod>) {
    mods.push(super::native_hooks::NativeMod {
        id: ID.into(),
        title: format!("Gravestones {VERSION}"),
        description: "Your items and experience wait in a grave where you died. Right click it \
                      to get them back. By Pneumono (MIT)."
            .into(),
        keys: Vec::new(),
    });
}

/// `native_block_entity`: the grave block gets its block entity.
pub(super) fn block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == BLOCK && entity.is_none() {
        *entity = Some((ENTITY.to_owned(), Compound::new()));
    }
}

/// `native_block_entity_ticks`: a grave ticks, to age.
pub(super) fn ticks(block: &str, ticks: &mut bool) {
    if block == BLOCK {
        *ticks = true;
    }
}

/// `native_block_entity_container`: the grave's slots, with no menu and no face
/// a hopper could reach.
pub(super) fn container(block: &str, container: &mut Option<ModContainer>) {
    if block == BLOCK {
        *container = Some(ModContainer {
            slots: SLOTS,
            ..Default::default()
        });
    }
}

/// `native_block_entity_update_tag`: what a client needs to draw the grave, the owner.
pub(super) fn update_tag(entity: &ModBlockEntity, tag: &mut Option<Compound>) {
    if entity.id != ENTITY {
        return;
    }
    let mut out = Compound::new();
    for name in ["owner", "owner_name"] {
        if let Some(value) = entity.nbt.get(&key(name)) {
            out.insert(key(name), value.clone());
        }
    }
    *tag = Some(out);
}

/// Java's `String.hashCode`.
fn java_hash(text: &str) -> i32 {
    text.encode_utf16()
        .fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// `native_block_entity_submit`: the owner's head, lying in front of the stone as
/// `AbstractGravestoneBlockEntityRenderer.renderPositionedHead` puts it: its turn and
/// tilt come from the grave's position.
pub(super) fn submit(
    game: &Game<'_>,
    pos: [i32; 3],
    entity: &ModBlockEntity,
    draw: &mut BlockEntityDraw,
) {
    if entity.id != ENTITY || !game.native_on(ID) {
        return;
    }
    let mut profile: Vec<(&str, Tag)> = Vec::new();
    if let Some(name) = get_text(&entity.nbt, "owner_name") {
        profile.push(("name", Tag::String(JavaString::from(name.as_str()))));
    }
    if let Some(id) = get_ints(&entity.nbt, "owner") {
        profile.push(("id", Tag::IntArray(id)));
    }
    let mut head = ItemStack::new("minecraft:player_head", 1);
    if !profile.is_empty() {
        head.data_mut()
            .set_other("minecraft:profile", Some(Tag::compound(profile)));
    }
    let text = format!(
        "Pos: BlockPos{{x={}, y={}, z={}}}, Some other text, I don't know, it doesn't matter what goes here.",
        pos[0], pos[1], pos[2]
    );
    let mut random = LegacyRandom::new(i64::from(java_hash(&text)));
    let yaw = random.next_float() * 70.0 - 35.0;
    let pitch = random.next_float() * -30.0;
    draw.push();
    draw.translate([0.5, 0.25, 0.475]);
    draw.rotate_about([0.0, 1.0, 0.0], yaw, [0.0; 3]);
    draw.rotate_about([1.0, 0.0, 0.0], pitch, [0.0; 3]);
    game.native_draw_item(draw, &head, DisplayContext::Fixed, None, 0);
    draw.pop();
}

fn key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn get_int(nbt: &Compound, name: &str) -> i32 {
    match nbt.get(&key(name)) {
        Some(Tag::Int(v)) => *v,
        _ => 0,
    }
}

fn get_long(nbt: &Compound, name: &str) -> i64 {
    match nbt.get(&key(name)) {
        Some(Tag::Long(v)) => *v,
        _ => 0,
    }
}

fn get_ints(nbt: &Compound, name: &str) -> Option<Vec<i32>> {
    match nbt.get(&key(name)) {
        Some(Tag::IntArray(v)) => Some(v.clone()),
        _ => None,
    }
}

fn get_text(nbt: &Compound, name: &str) -> Option<String> {
    match nbt.get(&key(name)) {
        Some(Tag::String(v)) => Some(v.to_string_lossy()),
        _ => None,
    }
}

/// `GravestoneDecay.calculateDecayStage`: 0, 1 and 2 are the three looks, 3 is broken.
fn decay_stage(total_damage: i32) -> i32 {
    if total_damage >= DAMAGE_TO_BREAK {
        3
    } else if total_damage <= 0 {
        0
    } else if total_damage + 1 == DAMAGE_TO_BREAK {
        2
    } else {
        3 * total_damage / DAMAGE_TO_BREAK
    }
}

/// The age damage a grave of this age has: one for each `DECAY_TICKS` passed, to three.
fn age_damage(age_ticks: i64) -> i32 {
    if age_ticks > DECAY_TICKS * 3 {
        3
    } else if age_ticks > DECAY_TICKS * 2 {
        2
    } else if age_ticks > DECAY_TICKS {
        1
    } else {
        0
    }
}

/// `native_block_entity_tick`: once a second, the grave ages (`GravestoneDecay.timeDecayGravestone`).
pub(super) fn tick(game: &mut Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    if entity.id != ENTITY || !game.native_on(ID) {
        return;
    }
    let now = game.world.game_time;
    if now % 20 != 0 {
        return;
    }
    // A grave with no owner never decays.
    if get_ints(&entity.nbt, "owner").is_none() {
        return;
    }
    let aged = age_damage(now - get_long(&entity.nbt, "spawn_ticks"))
        .max(get_int(&entity.nbt, "age_damage"));
    if aged != get_int(&entity.nbt, "age_damage") {
        entity.nbt.insert(key("age_damage"), Tag::Int(aged));
    }
    let stage = decay_stage(aged + get_int(&entity.nbt, "death_damage"));
    if stage >= 3 {
        game.gravestone_release(pos, entity);
        let dimension = game.dim.id().to_owned();
        if let Some(air) = air_state(game) {
            game.native_set_block(&dimension, pos, air, 3);
        }
    } else {
        let shown = game.world.block(pos[0], pos[1], pos[2]);
        let current = game
            .native_block_state_property(shown, "damage")
            .and_then(|v| v.parse::<i32>().ok());
        if current != Some(stage) {
            game.native_set_block_property_with(pos, "damage", &stage.to_string(), 3);
        }
    }
}

fn air_state(game: &Game<'_>) -> Option<horizon_world::registry::BlockStateId> {
    game.pack.assets.registry.default_state("minecraft:air")
}

/// `native_right_click_block`: a right click on a grave collects it. The click is
/// taken, so the player's item and hand do nothing else.
pub(super) fn right_click(game: &mut Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    if !game.native_on(ID) {
        return;
    }
    let state = game.world.block(pos[0], pos[1], pos[2]);
    if &*game.res.facts().get(state).name != BLOCK {
        return;
    }
    *canceled = true;
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return;
    }
    game.gravestone_collect(pos);
}

impl Game<'_> {
    /// `GravestonePlacement`: the cost of putting the grave at `at` when the player died at
    /// `origin`, or none when it cannot go there.
    fn gravestone_cost(&self, at: [i32; 3], origin: [i32; 3]) -> Option<f64> {
        let [x, y, z] = at;
        let shape = &self.world.shape;
        if y < shape.min_y || y >= shape.max_y() || !self.world.loaded(x, z) {
            return None;
        }
        let facts = self.res.facts();
        let state = self.world.block(x, y, z);
        let f = facts.get(state);
        // `GravestonePlacement.isInvalid`: unbreakable, immune to explosions, void air, or a grave.
        if f.hardness < 0.0
            || f.explosion_resistance >= 3_600_000.0
            || &*f.name == "minecraft:void_air"
            || &*f.name == BLOCK
        {
            return None;
        }
        let mut cost = match f.fluid {
            Fluid::None => 0.0,
            Fluid::Water => 1.0,
            _ => 5.0,
        };
        if !facts.is_air(state) {
            cost += if f.replaceable { 0.5 } else { 5.0 };
        }
        // Below the world counts as air, as vanilla's void air does.
        if y - 1 < shape.min_y || facts.is_air(self.world.block(x, y - 1, z)) {
            cost += 1.0;
        }
        let d = [x - origin[0], y - origin[1], z - origin[2]].map(f64::from);
        Some(cost + d[0] * d[0] + d[1] * d[1] + d[2] * d[2])
    }

    /// `GravestonePlacement.getNewValidPos`: where the grave goes for a death at `origin`.
    fn gravestone_position(&self, origin: [i32; 3]) -> Option<[i32; 3]> {
        if self.gravestone_cost(origin, origin) == Some(0.0) {
            return Some(origin);
        }
        let mut best: Option<(f64, [i32; 3])> = None;
        for x in -RADIUS..=RADIUS {
            for y in -RADIUS..=RADIUS {
                for z in -RADIUS..=RADIUS {
                    let at = [origin[0] + x, origin[1] + y, origin[2] + z];
                    if let Some(cost) = self.gravestone_cost(at, origin)
                        && best.is_none_or(|(b, _)| cost < b)
                    {
                        best = Some((cost, at));
                    }
                }
            }
        }
        best.map(|(_, at)| at)
    }

    /// `GravestoneCreation.create`, at the head of the death drops: the grave takes the
    /// player's items and experience. `at` is where the drops start, the eyes less 0.3.
    /// What it leaves (items that vanish with death) is dropped or destroyed as always.
    pub(super) fn gravestone_death(&mut self, at: [f64; 3]) {
        if !self.native_on(ID)
            || self.ui.rules.bool("keep_inventory")
            || (self.serving.is_none() && self.options.dedicated.is_none())
        {
            return;
        }
        let Some(grave) = self.pack.assets.registry.default_state(BLOCK) else {
            return;
        };
        // `player.blockPosition()`, kept inside the world as `getPlacementPos` does.
        let (min_y, max_y) = (self.world.shape.min_y, self.world.shape.max_y());
        let death = [
            at[0].floor() as i32,
            ((at[1] - 1.32).floor() as i32).clamp(min_y, max_y),
            at[2].floor() as i32,
        ];
        let Some(pos) = self.gravestone_position(death) else {
            return;
        };
        let dimension = self.dim.id().to_owned();
        if !self.native_set_block(&dimension, pos, grave, 3) {
            return;
        }

        // Take the items, by slot. Items that vanish stay: they are destroyed with the rest.
        let mut stored: Vec<Option<ItemStack>> = vec![None; SLOTS];
        let keeps = |s: &ItemStack| s.enchantment_level("minecraft:vanishing_curse") > 0;
        for (i, slot) in self.inventory.iter_mut().enumerate().take(36) {
            if slot.as_ref().is_some_and(|s| !keeps(s)) {
                stored[i] = slot.take();
            }
        }
        // `self.armor` runs head to feet; the grave counts feet to head, as `Inventory` does.
        for (a, slot) in self.armor.iter_mut().enumerate() {
            if slot.as_ref().is_some_and(|s| !keeps(s)) {
                stored[36 + (3 - a)] = slot.take();
            }
        }
        if self.offhand.as_ref().is_some_and(|s| !keeps(s)) {
            stored[40] = self.offhand.take();
        }
        self.apply_equipment();

        // `ExperienceDataType`: what vanilla would drop, at most 100, and none left to drop.
        let points = (self.experience.death_reward(false) as i32).min(EXPERIENCE_CAP);
        self.experience = Experience::default();

        let owner = self.acting_player_uuid();
        let name = self.acting_player_name().to_owned();
        let spawned = self.world.game_time;
        let filled = self.native_with_mod_block_entity(pos, |entity| {
            entity.items = stored.clone();
            entity.nbt.insert(key("experience"), Tag::Int(points));
            entity
                .nbt
                .insert(key("owner"), Tag::IntArray(owner.to_vec()));
            entity.nbt.insert(
                key("owner_name"),
                Tag::String(JavaString::from(name.as_str())),
            );
            entity.nbt.insert(key("spawn_ticks"), Tag::Long(spawned));
            entity.nbt.insert(key("death_damage"), Tag::Int(0));
            entity.nbt.insert(key("age_damage"), Tag::Int(0));
        });
        if filled.is_none() {
            // No block entity to keep them: nothing is lost, they drop as they would.
            for stack in stored.into_iter().flatten() {
                self.spawn_item(stack, at, [0.0, 0.2, 0.0], 40);
            }
            self.spawn_orbs(at, points as u32);
        }
        self.native_block_entity_sync(pos);
    }

    /// `GravestoneCollection.collect`: the grave gives its things to the acting player.
    fn gravestone_collect(&mut self, pos: [i32; 3]) {
        let me = self.acting_player_uuid();
        let uuid = self.native_acting_id();
        let taken = self.native_with_mod_block_entity(pos, |entity| {
            let owner = get_ints(&entity.nbt, "owner");
            let name = get_text(&entity.nbt, "owner_name").unwrap_or_else(|| "???".into());
            // `gravestone_accessible_owner_only`: only the owner opens a grave that has one.
            if owner.as_ref().is_some_and(|o| o[..] != me[..]) {
                return Err(name);
            }
            let items = std::mem::take(&mut entity.items);
            let points = get_int(&entity.nbt, "experience");
            entity.items = vec![None; items.len()];
            entity.nbt.insert(key("experience"), Tag::Int(0));
            Ok((items, points))
        });
        let (items, points) = match taken {
            Some(Ok(taken)) => taken,
            Some(Err(owner)) => {
                self.native_action_bar(
                    uuid,
                    Text::translate(
                        "gravestones.cannot_open_wrong_player",
                        &[Text::literal(owner)],
                    ),
                );
                return;
            }
            None => return,
        };
        let here = [
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.5,
            f64::from(pos[2]) + 0.5,
        ];
        let mut rest: Vec<ItemStack> = Vec::new();
        for (i, stack) in items.into_iter().enumerate() {
            let Some(stack) = stack else { continue };
            // Each stack goes back to the slot it came from, when that is free.
            let slot = match i {
                0..36 => self.inventory.get_mut(i),
                36..40 => self.armor.get_mut(3 - (i - 36)),
                _ => None,
            };
            match slot {
                Some(slot) if slot.is_none() => *slot = Some(stack),
                _ if i == 40 && self.offhand.is_none() => self.offhand = Some(stack),
                _ => rest.push(stack),
            }
        }
        self.apply_equipment();
        // What found its slot taken goes to any free slot, else drops at the grave.
        for stack in rest {
            match self.inventory.iter_mut().find(|s| s.is_none()) {
                Some(slot) => *slot = Some(stack),
                None => self.spawn_item(stack, here, [0.0, 0.2, 0.0], 10),
            }
        }
        if points > 0 {
            self.experience.give(points);
        }
        let dimension = self.dim.id().to_owned();
        if let Some(air) = air_state(self) {
            self.native_set_block(&dimension, pos, air, 3);
        }
    }

    /// `AbstractGravestoneBlockEntity` breaking: the things in a grave fall out where it stood.
    fn gravestone_release(&mut self, pos: [i32; 3], entity: &mut ModBlockEntity) {
        let here = [
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.5,
            f64::from(pos[2]) + 0.5,
        ];
        for stack in std::mem::take(&mut entity.items).into_iter().flatten() {
            self.spawn_item(stack, here, [0.0, 0.2, 0.0], 10);
        }
        let points = get_int(&entity.nbt, "experience");
        if points > 0 {
            self.spawn_orbs(here, points as u32);
        }
        entity.nbt.insert(key("experience"), Tag::Int(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_stage_follows_the_default_settings() {
        // damage_to_break 3: untouched, then chipped, then damaged, then gone.
        assert_eq!([0, 1, 2, 3, 4].map(decay_stage), [0, 1, 2, 3, 3]);
    }

    #[test]
    fn java_hash_matches_javas_string_hash_code() {
        // "hello".hashCode() in Java.
        assert_eq!(java_hash("hello"), 99_162_322);
        assert_eq!(java_hash(""), 0);
    }

    #[test]
    fn age_damage_is_one_per_decay_time() {
        assert_eq!(age_damage(DECAY_TICKS), 0);
        assert_eq!(age_damage(DECAY_TICKS + 1), 1);
        assert_eq!(age_damage(DECAY_TICKS * 2 + 1), 2);
        assert_eq!(age_damage(DECAY_TICKS * 3 + 1), 3);
    }
}
