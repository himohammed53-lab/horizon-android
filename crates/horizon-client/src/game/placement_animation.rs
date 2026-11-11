// A Good Place for Horizon.
//
// Ported from A Good Place 1.20-1.2.7 by MehVahdJukaar and Enjarai (MIT),
// https://modrinth.com/mod/a-good-place (source: https://github.com/enjarai/a-good-place).
// The animation follows that mod's `ConfiguredPlacingParticle` (the slide, the rotation, the
// scale and the height scale, each on `fancyExponent`'s curve) with the values of its
// `default_animations/all_blocks.json`; the neighbour check that keeps a block from sliding
// out of a wall is its `adjustDirectionBasedOnNeighbors`.
//
// A block the player places pops in: it starts a little off its cell, turned and lifted, and
// settles into place over three ticks. The section mesh leaves the block out while that
// plays (`mesher::hidden`) and the block is drawn here instead, through a matrix. It does
// nothing when the "A Good Place" mod is off in Options > Mods (it is off until turned on),
// for double blocks (beds, doors, tall plants, double chests), for blocks without a
// collision shape, and for blocks drawn by their block entity (chests, signs, banners...).
// Resource-pack animation files are not read: the one default animation is built in.

use super::{Game, Hand};
use glam::{Mat4, Vec3};
use horizon_client::blocks::RenderShape;
use horizon_core::geometry::{Axis, Direction};
use horizon_world::registry::BlockStateId;
use std::{collections::HashMap, time::Instant};

pub(super) const ID: &str = "a_good_place";
/// Size of the block that fills the cell under the animation (a hair under 1 so it never fights the final pose).
const BACKING_SCALE: f32 = 0.995;
/// Brightness of the animated block against the chunk mesh's (no ambient occlusion on it).
const DIM: u32 = 230;

/// `color` (ABGR, alpha kept) with its red, green and blue scaled by `DIM / 255`.
fn dim(color: u32) -> u32 {
    let channel = |shift: u32| ((color >> shift & 255) * DIM / 255) << shift;
    color & 0xFF00_0000 | channel(16) | channel(8) | channel(0)
}
pub(super) const VERSION: &str = "1.2.7";

/// `duration`: the animation's length in ticks.
const DURATION: f32 = 3.0;
/// `ConfiguredPlacingParticle.extraLifeTicks`: it is drawn this much longer than it hides the
/// block, so the rebuilt mesh has time to show the block before the animation goes.
const EXTRA_LIFE: f32 = 2.0;
/// `scale` and `scale_curve`.
const SCALE_START: f32 = 1.0;
const SCALE_CURVE: f32 = -0.7;
/// `translation` (x to the player's side, y up, z toward the player) and its curve.
const TRANSLATION: [f32; 3] = [0.1, 0.2, 0.2];
const TRANSLATION_CURVE: f32 = 0.9;
/// `rotation` in degrees and `rotation_curve`.
const ROTATION_DEGREES: [f32; 3] = [0.0, -5.7, 5.7];
const ROTATION_CURVE: f32 = -0.08;
/// `rotation_pivot`, relative to the block's centre.
const PIVOT: [f32; 3] = [0.5, -0.5, 0.5];
/// `height` and `height_curve`.
const HEIGHT_START: f32 = 1.0;
const HEIGHT_CURVE: f32 = 0.0;
/// The block ends a little below full size so it never clips the block that replaces it.
const END_SCALE: f32 = 0.999;

/// `PlacingBlockParticle`: one block on show.
struct Animation {
    state: BlockStateId,
    started: Instant,
    /// The block still hidden from the section mesh.
    hidden: bool,
    /// `yAngle`.
    y_angle: f32,
    /// `slideStart`: the z and y offsets, relative to `y_angle`.
    slide_start: [f32; 2],
    /// `rotStart`, in radians.
    rotation_start: [f32; 3],
    /// `BlockState.getOffset`: the model's jitter, for flowers and the like.
    offset: [f32; 3],
}

/// `BlocksParticlesManager.PARTICLES`.
#[derive(Default)]
pub(super) struct PlacementAnimations {
    shown: HashMap<[i32; 3], Animation>,
}

impl PlacementAnimations {
    pub(super) fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }
}

/// `AnimationParameters.fancyExponent`: an exponential from 0 to 1 whose bend `curve` (in
/// -1..1) sets: 0 is a straight line, towards 1 it starts slow and ends steep, towards -1 it
/// starts steep and ends slow.
fn fancy_exponent(t: f32, curve: f32) -> f32 {
    if curve == 0.0 {
        return t;
    }
    let base = if curve > 0.0 {
        -curve.ln()
    } else {
        (-curve).ln() - 1.0
    };
    base * (1.0 / base + 1.0).powf(t) - base
}

/// `Direction.orderedByNearest(entity)`: the six directions, the one the player looks along
/// most first.
fn ordered_by_nearest(yaw: f32, pitch: f32) -> [Direction; 6] {
    let pitch_rad = pitch * (std::f32::consts::PI / 180.0);
    let yaw_rad = -yaw * (std::f32::consts::PI / 180.0);
    let (pitch_sin, pitch_cos) = pitch_rad.sin_cos();
    let (yaw_sin, yaw_cos) = yaw_rad.sin_cos();
    let x_pos = yaw_sin > 0.0;
    let y_pos = pitch_sin < 0.0;
    let z_pos = yaw_cos > 0.0;
    let x_yaw = if x_pos { yaw_sin } else { -yaw_sin };
    let y_mag = if y_pos { -pitch_sin } else { pitch_sin };
    let z_yaw = if z_pos { yaw_cos } else { -yaw_cos };
    let x_mag = x_yaw * pitch_cos;
    let z_mag = z_yaw * pitch_cos;
    let ax = if x_pos {
        Direction::East
    } else {
        Direction::West
    };
    let ay = if y_pos {
        Direction::Up
    } else {
        Direction::Down
    };
    let az = if z_pos {
        Direction::South
    } else {
        Direction::North
    };
    let make = |a: Direction, b: Direction, c: Direction| {
        [a, b, c, c.opposite(), b.opposite(), a.opposite()]
    };
    if x_yaw > z_yaw {
        if y_mag > x_mag {
            make(ay, ax, az)
        } else if z_mag > y_mag {
            make(ax, az, ay)
        } else {
            make(ax, ay, az)
        }
    } else if y_mag > z_mag {
        make(ay, az, ax)
    } else if x_mag > y_mag {
        make(az, ax, ay)
    } else {
        make(az, ay, ax)
    }
}

/// The step of a direction as a vector.
fn step(direction: Direction) -> Vec3 {
    let [x, y, z] = direction.normal();
    Vec3::new(x as f32, y as f32, z as f32)
}

/// The slide the block starts with, as `ConfiguredPlacingParticle`'s constructor works it
/// out: `(yAngle, slideStart)`. `yaw` and `pitch` are the placer's look in degrees,
/// `hand_side` is 1 for a right hand and -1 for a left, and `free[d]` says whether the
/// neighbour on side `d` (in `Direction::ALL` order) leaves room to slide that way.
fn slide(yaw: f32, pitch: f32, hand_side: f32, free: [bool; 6]) -> (f32, [f32; 2]) {
    // `playerHorizLook`: the horizontal look turned round, towards the player.
    let (yaw_sin, yaw_cos) = yaw.to_radians().sin_cos();
    let toward = Vec3::new(yaw_sin, 0.0, -yaw_cos);
    // The player's hand side, then the change of basis (x: hand side, y: up, z: toward).
    let right = Vec3::new(toward.z, 0.0, -toward.x) * hand_side;
    let mut translation = Vec3::from(TRANSLATION);
    let power = translation.length();
    if pitch <= 0.0 {
        translation.y = -translation.y;
    }
    let mut direction = right * translation.x + Vec3::Y * translation.y + toward * translation.z;

    // `adjustDirectionBasedOnNeighbors` (restrict_direction is on): no sliding into a block.
    let mut affected = Vec::new();
    if direction.z > 0.0 {
        affected.push(Direction::South);
    }
    if direction.z < 0.0 {
        affected.push(Direction::North);
    }
    if direction.x > 0.0 {
        affected.push(Direction::East);
    }
    if direction.x < 0.0 {
        affected.push(Direction::West);
    }
    if direction.y > 0.0 {
        affected.push(Direction::Up);
    }
    if direction.y < 0.0 {
        affected.push(Direction::Down);
    }
    for d in affected {
        if !free[d as usize] {
            let mask = step(d).abs();
            direction -= mask * direction;
        }
    }
    if direction.length() == 0.0 {
        let nearest = ordered_by_nearest(yaw, pitch);
        let rank = |d: &Direction| nearest.iter().position(|n| n == d).unwrap_or(usize::MAX);
        let mut room: Vec<Direction> = Direction::ALL
            .into_iter()
            .filter(|d| free[*d as usize])
            .collect();
        room.sort_by_key(rank);
        if let Some(d) = room.first() {
            direction = step(*d);
        }
    }
    if direction.length() == 0.0 {
        // Walled in on every side: nothing to slide from (the mod's own maths gives NaN).
        return (0.0, [0.0, 0.0]);
    }

    let unit = direction.normalize();
    let y_angle = direction.x.atan2(direction.z);
    // `animationDirection.yRot(-yAngle)`, its z and y scaled by the slide's power.
    let (sin, cos) = (-y_angle).sin_cos();
    let z = unit.z * cos - unit.x * sin;
    (y_angle, [z * power, unit.y * power])
}

/// `applyAnimation`: the pose at `time` (0 to 1) as a matrix on the block's model in 0..1
/// block coordinates, before it is moved to the block's cell.
fn pose(
    y_angle: f32,
    slide_start: [f32; 2],
    rotation_start: [f32; 3],
    offset: [f32; 3],
    time: f32,
) -> Mat4 {
    let mut m = Mat4::from_translation(Vec3::splat(0.5));
    let offset = Vec3::from(offset);
    m *= Mat4::from_translation(offset);
    // Everything below is relative to the direction of the slide.
    m *= Mat4::from_rotation_y(y_angle);

    // Slide.
    let progress = fancy_exponent(time, TRANSLATION_CURVE);
    m *= Mat4::from_translation(Vec3::new(
        0.0,
        slide_start[1] * (1.0 - progress),
        slide_start[0] * (1.0 - progress),
    ));

    // Rotation about a pivot on the block's edge towards the slide.
    let progress = fancy_exponent(time, ROTATION_CURVE);
    let mut rotation = Vec3::from(rotation_start) * (1.0 - progress);
    let mut pivot = Vec3::from(PIVOT);
    if slide_start[1] < 0.0 {
        pivot.y = -pivot.y;
        rotation *= Vec3::new(-1.0, 1.0, -1.0);
    }
    m *= Mat4::from_translation(pivot);
    m *= Mat4::from_rotation_y(rotation.y);
    m *= Mat4::from_rotation_z(rotation.z);
    m *= Mat4::from_rotation_x(rotation.x);
    m *= Mat4::from_translation(-pivot);

    // Scale, a little under 1 at the end.
    let progress = fancy_exponent(time, SCALE_CURVE);
    let scale = SCALE_START + (END_SCALE - SCALE_START) * progress;
    m *= Mat4::from_scale(Vec3::splat(scale));

    // Height scale, about the block's base.
    let progress = fancy_exponent(time, HEIGHT_CURVE);
    let height = HEIGHT_START + (1.0 - HEIGHT_START) * progress;
    m *= Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0));
    m *= Mat4::from_scale(Vec3::new(1.0, height, 1.0));
    m *= Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0));

    // Back to the block's own frame.
    m *= Mat4::from_rotation_y(-y_angle);
    m *= Mat4::from_translation(-offset);
    m * Mat4::from_translation(Vec3::splat(-0.5))
}

/// The mod's row in Options > Mods.
pub(super) fn list(mods: &mut Vec<super::native_hooks::NativeMod>) {
    mods.push(super::native_hooks::NativeMod {
        id: ID.into(),
        title: format!("A Good Place {VERSION}"),
        description: "Blocks you place pop in with a small animation. By MehVahdJukaar and \
                      Enjarai (MIT)."
            .into(),
        keys: Vec::new(),
    });
}

impl Game<'_> {
    /// Whether the player turned the mod on: it is off until they do.
    fn placement_animation_on(&self) -> bool {
        self.settings.mods.enabled.contains(ID) && !self.settings.mods.disabled.contains(ID)
    }

    /// `IsDoubleBlock`: beds, doors, tall plants and double chests keep still.
    fn is_double_block(&self, state: BlockStateId) -> bool {
        let Some(info) = self.pack.assets.registry.state(state) else {
            return false;
        };
        let property = |name: &str| info.properties.get(name).map(String::as_str);
        let name = self.res.facts().get(state).name.clone();
        matches!(property("half"), Some("upper" | "lower"))
            || (name.ends_with("_bed") && property("part").is_some())
            || (name.contains("chest") && matches!(property("type"), Some("left" | "right")))
    }

    /// `BlocksParticlesManager.addParticle`, for the block the player has just placed at `pos`
    /// (already set on this client's level).
    pub(super) fn start_placement_animation(&mut self, pos: [i32; 3], hand: Hand) {
        if !self.placement_animation_on() {
            return;
        }
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let facts = self.res.facts();
        if facts.is_air(state) || facts.get(state).render_shape != RenderShape::Model {
            return;
        }
        // The default animation's predicates: not a double block, and with a collision shape.
        if self.is_double_block(state) || self.block_collision_shape(pos).is_empty() {
            return;
        }
        // Room to slide towards: `emptyDirections`.
        let free: [bool; 6] = std::array::from_fn(|i| {
            let d = Direction::ALL[i];
            let [dx, dy, dz] = d.normal();
            let shape = self.block_collision_shape([pos[0] + dx, pos[1] + dy, pos[2] + dz]);
            if shape.is_empty() {
                return true;
            }
            let axis = match d {
                Direction::Down | Direction::Up => Axis::Y,
                Direction::North | Direction::South => Axis::Z,
                Direction::West | Direction::East => Axis::X,
            };
            if matches!(d, Direction::Up | Direction::South | Direction::East) {
                shape.min(axis) > 0.25
            } else {
                shape.max(axis) < 0.75
            }
        });
        let mut hand_side = if self.settings.main_hand_left {
            -1.0
        } else {
            1.0
        };
        if hand != Hand::Main {
            hand_side = -hand_side;
        }
        let (y_angle, slide_start) = slide(self.yaw, self.pitch, hand_side, free);
        let offset = self
            .pack
            .context
            .tables
            .offset(state, pos[0], pos[2])
            .map_or([0.0; 3], |o| [o[0] as f32, o[1] as f32, o[2] as f32]);
        horizon_client::mesher::hidden::hide(pos);
        self.placement_animations.shown.insert(
            pos,
            Animation {
                state,
                started: Instant::now(),
                hidden: true,
                y_angle,
                slide_start,
                rotation_start: ROTATION_DEGREES.map(f32::to_radians),
                offset,
            },
        );
    }

    /// `BlocksParticlesManager.renderParticles`, and the particles' tick: puts each block back
    /// in the mesh once its animation has run (or the block changed), and draws the ones still
    /// playing, camera-relative.
    pub(super) fn placement_animation_geometry(
        &mut self,
        camera: glam::DVec3,
        out: &mut Vec<horizon_client::mesher::Vertex>,
    ) {
        if self.placement_animations.is_empty() {
            return;
        }
        let now = Instant::now();
        let mut shown = std::mem::take(&mut self.placement_animations.shown);
        let mut show_again = Vec::new();
        shown.retain(|pos, a| {
            // `age + partialTicks`, in ticks.
            let age = now.duration_since(a.started).as_secs_f32() * 20.0;
            let changed = self.world.block(pos[0], pos[1], pos[2]) != a.state;
            if a.hidden && (changed || age >= DURATION) {
                a.hidden = false;
                show_again.push(*pos);
            }
            !(changed || age >= DURATION + EXTRA_LIFE)
        });
        for pos in show_again {
            if horizon_client::mesher::hidden::show(pos) {
                self.world.mark_block_dirty(pos[0], pos[1], pos[2]);
            }
        }
        for (pos, a) in &shown {
            let age = now.duration_since(a.started).as_secs_f32() * 20.0;
            let time = (age / (DURATION + 1.0)).min(1.0);
            let at =
                glam::DVec3::new(f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2])) - camera;
            let block = horizon_core::position::BlockPos::new(pos[0], pos[1], pos[2]);
            let start = out.len();
            // The cell is empty in the chunk mesh while the block moves, and its neighbours don't
            // draw their faces towards it: a block just inside the cell fills the hole (no black).
            let backing = Mat4::from_translation(at.as_vec3())
                * Mat4::from_translation(Vec3::splat(0.5))
                * Mat4::from_scale(Vec3::splat(BACKING_SCALE))
                * Mat4::from_translation(Vec3::splat(-0.5));
            self.moving_block_vertices_with(a.state, block, backing, out);
            let matrix = Mat4::from_translation(at.as_vec3())
                * pose(a.y_angle, a.slide_start, a.rotation_start, a.offset, time);
            self.moving_block_vertices_with(a.state, block, matrix, out);
            // The chunk mesh darkens corners (ambient occlusion); this has none, so it is a little dimmer.
            for v in &mut out[start..] {
                v.color = dim(v.color);
            }
        }
        // A block placed during this call's drawing would be lost: merge what arrived.
        shown.extend(std::mem::take(&mut self.placement_animations.shown));
        self.placement_animations.shown = shown;
    }

    /// A world closed or another opened: nothing stays hidden.
    pub(super) fn clear_placement_animations(&mut self) {
        self.placement_animations.shown.clear();
        horizon_client::mesher::hidden::clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn dimming_keeps_alpha_and_scales_each_channel() {
        assert_eq!(dim(0xFF00_0000), 0xFF00_0000);
        assert_eq!(dim(0xFFFF_FFFF), 0xFFE6_E6E6);
        assert_eq!(dim(0x80FF_8000), 0x80E6_7300);
    }

    #[test]
    fn the_exponential_runs_from_zero_to_one_whatever_its_bend() {
        for curve in [-0.9, -0.7, -0.08, 0.0, 0.5, 0.9] {
            assert!(fancy_exponent(0.0, curve).abs() < 1e-5, "{curve}");
            assert!((fancy_exponent(1.0, curve) - 1.0).abs() < 1e-5, "{curve}");
        }
        // A negative bend starts steep (ahead of the line), a positive one starts slow.
        assert!(fancy_exponent(0.25, -0.7) > 0.25);
        assert!(fancy_exponent(0.25, 0.9) < 0.25);
        assert_eq!(fancy_exponent(0.3, 0.0), 0.3);
    }

    #[test]
    fn at_the_end_the_block_sits_in_its_cell_a_hair_smaller() {
        let (y_angle, slide_start) = slide(37.0, 12.0, 1.0, [true; 6]);
        let end = pose(
            y_angle,
            slide_start,
            ROTATION_DEGREES.map(f32::to_radians),
            [0.0; 3],
            1.0,
        );
        // The centre stays put and a corner is pulled in by (1 - 0.999) / 2 of the cell.
        assert!(close(
            end.transform_point3(Vec3::splat(0.5)),
            Vec3::splat(0.5)
        ));
        assert!(close(end.transform_point3(Vec3::ZERO), Vec3::splat(0.0005)));
        assert!(close(end.transform_point3(Vec3::ONE), Vec3::splat(0.9995)));
    }

    #[test]
    fn the_slide_is_as_long_as_the_translation_when_nothing_blocks_it() {
        let length = Vec3::from(TRANSLATION).length();
        for (yaw, pitch) in [(0.0, 0.0), (90.0, 30.0), (-135.0, -40.0), (200.0, 5.0)] {
            let (_, [z, y]) = slide(yaw, pitch, 1.0, [true; 6]);
            assert!((z.hypot(y) - length).abs() < 1e-5, "{yaw} {pitch}");
        }
    }

    #[test]
    fn a_block_slides_in_from_the_players_side_and_below_when_looking_level() {
        // Facing south (yaw 0) the player is to the north, so the block starts north, to the
        // player's right (west) and lower: (-0.1, -0.2, -0.2) before the neighbours are asked.
        let (y_angle, [z, y]) = slide(0.0, 0.0, 1.0, [true; 6]);
        let start = Mat4::from_rotation_y(y_angle).transform_vector3(Vec3::new(0.0, y, z));
        assert!(close(start, Vec3::new(-0.1, -0.2, -0.2)), "{start}");
        // A left hand puts the sideways part on the other side.
        let (y_angle, [z, y]) = slide(0.0, 0.0, -1.0, [true; 6]);
        let start = Mat4::from_rotation_y(y_angle).transform_vector3(Vec3::new(0.0, y, z));
        assert!(close(start, Vec3::new(0.1, -0.2, -0.2)), "{start}");
    }

    #[test]
    fn a_neighbour_in_the_way_removes_that_part_of_the_slide() {
        // The same placement with a solid block below: the downward part is dropped.
        let mut free = [true; 6];
        free[Direction::Down as usize] = false;
        let (y_angle, [z, y]) = slide(0.0, 0.0, 1.0, free);
        assert_eq!(y, 0.0);
        let start = Mat4::from_rotation_y(y_angle).transform_vector3(Vec3::new(0.0, y, z));
        // Length is still the full translation; only the direction (-0.1, 0, -0.2) is kept.
        let want = Vec3::new(-0.1, 0.0, -0.2).normalize() * Vec3::from(TRANSLATION).length();
        assert!(close(start, want), "{start} {want}");
    }

    #[test]
    fn with_every_direction_but_one_blocked_the_block_comes_from_the_free_side() {
        let mut free = [false; 6];
        free[Direction::Up as usize] = true;
        let (y_angle, [z, y]) = slide(0.0, 0.0, 1.0, free);
        let start = Mat4::from_rotation_y(y_angle).transform_vector3(Vec3::new(0.0, y, z));
        let length = Vec3::from(TRANSLATION).length();
        assert!(close(start, Vec3::new(0.0, length, 0.0)), "{start}");
    }

    #[test]
    fn a_walled_in_block_does_not_slide() {
        assert_eq!(slide(10.0, 10.0, 1.0, [false; 6]), (0.0, [0.0, 0.0]));
    }

    #[test]
    fn the_nearest_direction_follows_where_the_player_looks() {
        // Looking level to the south: south first, north last.
        let order = ordered_by_nearest(0.0, 0.0);
        assert_eq!(order[0], Direction::South);
        assert_eq!(order[5], Direction::North);
        // Looking straight down: down first.
        assert_eq!(ordered_by_nearest(0.0, 90.0)[0], Direction::Down);
        // Looking east (yaw -90): east first.
        assert_eq!(ordered_by_nearest(-90.0, 0.0)[0], Direction::East);
    }
}
