//! The launcher's pixel sunset behind the section pages: the same scene as Horizon's own
//! launcher (its seeds, palette, sky stops, ridges, trees and sun slices), drawn still, as plain
//! rectangles. The scene's code is MIT-licensed; this is a small port of its still parts: no
//! clouds, meteor or fireflies, and the glow is a few soft ellipses.
use horizon_client::ui_theme::{Ui, color};
use std::sync::Mutex;

const SKY: [u32; 15] = [
    0x0a0919, 0x0f0c28, 0x151136, 0x1d1444, 0x281850, 0x361b5b, 0x481e66, 0x5d236e, 0x752873,
    0x902f76, 0xab3a75, 0xc64a71, 0xde5f6a, 0xef7c63, 0xf89d62,
];
const SKY_STEPS: usize = SKY.len() - 4;
const SUN: [u32; 7] = [
    0xfff5d2, 0xffe4a0, 0xffca78, 0xffac5f, 0xff8c58, 0xfb6c62, 0xee5277,
];
const HALO: u32 = 0xff8d6a;
const FOG: u32 = 0xe0697c;
const STAR_COOL: u32 = 0xd2c8ff;
const STAR_WARM: u32 = 0xfff3e4;
const MARGIN: i32 = 20;
const TREES: [&[&str]; 3] = [
    &[
        "..###..", ".#####.", "#######", "#######", ".#####.", "...#...", "...#...",
    ],
    &[
        "..#..", "..#..", ".###.", "..#..", ".###.", "#####", "..#..", "..#..",
    ],
    &[".###.", "#####", "#####", "..#..", "..#.."],
];

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}
fn smoothstep(a: f64, b: f64, v: f64) -> f64 {
    let t = clamp((v - a) / (b - a), 0., 1.);
    t * t * (3. - 2. * t)
}
fn round(v: f64) -> f64 {
    (v + 0.5).floor()
}
fn mix(a: u32, b: u32, t: f64) -> u32 {
    [16, 8, 0].iter().fold(0, |out, &s| {
        let (x, y) = (((a >> s) & 255) as f64, ((b >> s) & 255) as f64);
        out | ((x + (y - x) * t) as i64 as u32 & 255) << s
    })
}

struct Rng(i32);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let s = self.0;
        let mut t = (s ^ ((s as u32) >> 15) as i32).wrapping_mul(1 | s);
        t = t.wrapping_add((t ^ ((t as u32) >> 7) as i32).wrapping_mul(61 | t)) ^ t;
        ((t ^ ((t as u32) >> 14) as i32) as u32) as f64 / 4294967296.
    }
}

struct Noise([f32; 256]);
impl Noise {
    fn new(seed: i32) -> Noise {
        let mut rng = Rng(seed);
        Noise(std::array::from_fn(|_| rng.next() as f32))
    }
    fn at(&self, x: f64) -> f64 {
        let (mut value, mut amp, mut freq, mut norm) = (0., 0.5, 1., 0.);
        for _ in 0..4 {
            let p = x * freq;
            let i = p.floor();
            let s = p - i;
            let i = i as i64;
            let a = self.0[(i & 255) as usize] as f64;
            let b = self.0[((i + 1) & 255) as usize] as f64;
            value += (a + (b - a) * s * s * (3. - 2. * s)) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.1;
        }
        value / norm
    }
}

struct Star {
    x: f64,
    y: f64,
    size: f64,
    b: f64,
    cool: bool,
}

struct Ridge {
    top: Vec<i32>,
    color: u32,
    rim: u32,
    rim_strength: f64,
    block: i32,
    trees: Vec<(i32, usize)>,
}

pub struct Scene {
    px: f64,
    w: i32,
    h: i32,
    horizon: i32,
    fog_top: i32,
    sun: (i32, i32, i32),
    glow: Vec<f64>,
    stars: Vec<Star>,
    ridges: Vec<Ridge>,
}

impl Scene {
    fn new(width: f64, height: f64) -> Scene {
        let land_height = clamp(height * 0.3, 180., 320.);
        let (sun_x, sun_scale, sun_rise) = if width >= 1024. {
            (0.72, 1.1, 0.1)
        } else {
            (0.5, 1., 0.)
        };
        let px = clamp(round(width / 340.), 3., 6.);
        let w = (width / px).ceil() as i32;
        let h = (height / px).ceil() as i32;
        let land = land_height / px;
        let horizon = round(h as f64 - land * 0.55) as i32;
        let r = round(land * 0.4 * sun_scale) as i32;
        let sun = (
            round(w as f64 * sun_x) as i32,
            round(horizon as f64 - r as f64 * (0.45 + sun_rise)) as i32,
            r,
        );
        let fog_top = round(horizon as f64 - land * 0.18) as i32;
        let glow = (0..w)
            .map(|x| {
                let d = (x - sun.0) as f64 / (w as f64 * 0.3);
                (-d * d).exp()
            })
            .collect::<Vec<_>>();
        let mut rand = Rng(7);
        let sky_height = horizon as f64 * px;
        let star_count = round(width * sky_height / 5200.) as usize;
        let mut stars = Vec::with_capacity(star_count);
        for _ in 0..star_count {
            let x = rand.next() * width;
            let y = 2. + rand.next().powf(1.8) * sky_height * 0.75;
            let size = if rand.next() < 0.15 {
                1.8
            } else {
                1. + rand.next() * 0.4
            };
            let b = if rand.next() < 0.1 {
                0.8 + rand.next() * 0.2
            } else {
                0.25 + rand.next() * 0.4
            };
            let _phase = rand.next();
            let _speed = rand.next();
            let _sparkle = rand.next();
            let cool = rand.next() < 0.35;
            stars.push(Star {
                x,
                y,
                size,
                b,
                cool,
            });
        }
        // The website draws its clouds from the same generator, after the stars; the
        // ridges' trees come after them, so the draws are replayed to land on the same trees.
        let cols = w + MARGIN * 2;
        let ridge = |seed: i32,
                     base: f64,
                     amp: f64,
                     block: i32,
                     feature: f64,
                     valley: f64,
                     color: u32,
                     rim: u32,
                     rim_strength: f64| {
            let n = Noise::new(seed);
            let mut top = vec![0i32; cols as usize];
            let mut i = 0;
            while i < cols {
                let x = (i - MARGIN) as f64 + block as f64 / 2.;
                let d = (x - sun.0 as f64).abs() / (w as f64 * 0.5);
                let env = valley + (1. - valley) * (d * d * 1.8).min(1.);
                let v = clamp((n.at(x * px / feature) - 0.2) / 0.6, 0., 1.);
                let y = round((base - amp * env * v) / block as f64) as i32 * block;
                for k in 0..block {
                    if i + k < cols {
                        top[(i + k) as usize] = y;
                    }
                }
                i += block;
            }
            Ridge {
                top,
                color,
                rim,
                rim_strength,
                block,
                trees: Vec::new(),
            }
        };
        let hz = horizon as f64;
        let mut ridges = vec![
            ridge(11, hz, land * 0.3, 2, 170., 0.2, 0x3f1f5c, 0xc45a7f, 1.),
            ridge(
                23,
                hz + land * 0.22,
                land * 0.36,
                3,
                130.,
                0.35,
                0x25123e,
                0x7a3068,
                0.8,
            ),
            ridge(
                37,
                hz + land * 0.5,
                land * 0.38,
                4,
                100.,
                0.5,
                0x100a1f,
                0x2c1640,
                0.6,
            ),
        ];
        // Cloud draws (three big, five low) use the generator before the trees do.
        let _ = &mut rand;
        let mut col = MARGIN + (rand.next() * 12.).floor() as i32;
        while col < cols - 8 {
            let sprite = (rand.next() * TREES.len() as f64).floor() as usize;
            ridges[2].trees.push((col, sprite));
            col += 14 + (rand.next() * 30.).floor() as i32;
        }
        Scene {
            px,
            w,
            h,
            horizon,
            fog_top,
            sun,
            glow,
            stars,
            ridges,
        }
    }

    fn sky_row(&self, y: f64) -> u32 {
        let horizon = self.horizon as f64 * self.px;
        let t = clamp(y / horizon, 0., 1.);
        let f = t.powf(2.3) * SKY_STEPS as f64;
        let i = (f.floor() as usize).min(SKY_STEPS);
        let mut c = mix(SKY[i], SKY[(i + 1).min(SKY.len() - 1)], f - i as f64);
        let fog_top = self.fog_top as f64 * self.px;
        if y >= fog_top {
            let t = clamp((y - fog_top) / (horizon - fog_top), 0., 1.);
            c = mix(c, FOG, 0.3 * t);
        }
        c
    }
}

/// The sky's colour a share `t` (0 top, 1 horizon) of the way down: the same gradient as the
/// scene, for the home screen's hologram.
pub fn sky_at(t: f64) -> u32 {
    let f = clamp(t, 0., 1.).powf(2.3) * SKY_STEPS as f64;
    let i = (f.floor() as usize).min(SKY_STEPS);
    mix(SKY[i], SKY[(i + 1).min(SKY.len() - 1)], f - i as f64)
}

/// The scene's warm light and star colours, for the hologram's waves and dust.
pub const WAVE: u32 = 0xf3a6bd;
pub const DUST: u32 = 0xfff3e4;

static CACHE: Mutex<Option<((u32, u32), Scene)>> = Mutex::new(None);

/// Draws the sunset over the whole `w` × `h` window.
pub fn draw(ui: &mut Ui, w: f32, h: f32) {
    let key = (w.round() as u32, h.round() as u32);
    let mut cache = CACHE.lock().unwrap();
    if cache.as_ref().map(|c| c.0) != Some(key) {
        *cache = Some((key, Scene::new(w as f64, h as f64)));
    }
    let scene = &cache.as_ref().unwrap().1;
    let px = scene.px as f32;
    // The sky, a texel row at a time, down to the window's bottom (the land covers the rest).
    for row in 0..scene.h {
        let y = row as f32 * px;
        let c = scene.sky_row(y as f64 + px as f64 / 2.);
        ui.rounded(0.0, y, w, px + 1.0, 0.0, color::hex(c, 255));
    }
    // The glow of the sun on the horizon: a few soft ellipses.
    let (sx, hz) = (scene.sun.0 as f32 * px, scene.horizon as f32 * px);
    let ry = hz * 0.32;
    for k in 0..8 {
        let f = 1.0 - k as f32 / 8.0;
        let (rw, rh) = (w * 0.3 * f, ry * f);
        ui.rounded(
            sx - rw,
            hz - rh,
            rw * 2.0,
            rh * 2.0,
            rh,
            color::hex(SKY[14], (14.0 + k as f32 * 4.0) as u8),
        );
    }
    // The stars.
    for s in &scene.stars {
        let sky_height = hz as f64;
        let alpha = s.b * (1. - smoothstep(0.4, 0.82, s.y / sky_height)) * 0.65;
        if alpha < 0.04 {
            continue;
        }
        let c = if s.cool { STAR_COOL } else { STAR_WARM };
        let size = s.size as f32;
        ui.rounded(
            s.x as f32 - size / 2.0,
            s.y as f32 - size / 2.0,
            size,
            size,
            size / 2.0,
            color::hex(c, (alpha * 255.0) as u8),
        );
    }
    // The sun's halo, then the sun cut in slices.
    let (cx, cy, r) = (
        scene.sun.0 as f32 * px,
        scene.sun.1 as f32 * px,
        scene.sun.2 as f32 * px,
    );
    for k in 0..6 {
        let rr = r * (2.4 - k as f32 * 0.3);
        ui.rounded(
            cx - rr,
            cy - rr,
            rr * 2.0,
            rr * 2.0,
            rr,
            color::hex(HALO, 16),
        );
    }
    let (sx, sy, sr) = (scene.sun.0 as f64, scene.sun.1 as f64, scene.sun.2 as f64);
    let stripe = (sr / 5.).round().max(4.);
    let first = (sy - sr).ceil().max(0.) as i32;
    let last = ((sy + sr).floor() as i32).min(scene.h - 1);
    for y in first..=last {
        let dy = y as f64 + 0.5 - sy;
        let half = (sr * sr - dy * dy).sqrt();
        if half.is_nan() || half <= 0. {
            continue;
        }
        let v = (dy + sr) / (2. * sr);
        if v > 0.38 {
            let gap = 1. + (v - 0.38) / (1. - 0.38) * stripe * 0.6;
            if (y as f64 - sy).rem_euclid(stripe) < gap {
                continue;
            }
        }
        let f = v * (SUN.len() - 1) as f64;
        let i = f.floor() as usize;
        let c = mix(SUN[i], SUN[(i + 1).min(SUN.len() - 1)], f - i as f64);
        let from = round(sx - half).max(0.) as f32;
        let to = (round(sx + half) as f32).min(scene.w as f32);
        ui.rounded(
            from * px,
            y as f32 * px,
            (to - from) * px,
            px,
            0.0,
            color::hex(c, 255),
        );
    }
    // The three ridges, far to near, with their rim light and the trees.
    for ridge in &scene.ridges {
        let cols = ridge.top.len() as i32;
        let mut i = 0;
        while i < cols {
            let top = ridge.top[i as usize];
            let gx = (i - MARGIN + ridge.block / 2).clamp(0, scene.w - 1) as usize;
            let rim = mix(ridge.color, ridge.rim, scene.glow[gx] * ridge.rim_strength);
            let x = (i - MARGIN) as f32 * px;
            let bw = ridge.block as f32 * px;
            let y = top as f32 * px;
            ui.rounded(x, y, bw, h - y + 2.0, 0.0, color::hex(ridge.color, 255));
            ui.rounded(x, y, bw, px, 0.0, color::hex(rim, 255));
            i += ridge.block;
        }
        for &(col, sprite) in &ridge.trees {
            let ground = ridge.top[col as usize];
            let rows = TREES[sprite];
            let trunk = rows[rows.len() - 1].find('#').unwrap_or(0) as i32;
            for (ty, row) in rows.iter().enumerate() {
                let y = ground - rows.len() as i32 + ty as i32;
                for (tx, ch) in row.chars().enumerate() {
                    if ch == '#' {
                        let x = (col - MARGIN - trunk + tx as i32) as f32 * px;
                        ui.rounded(x, y as f32 * px, px, px, 0.0, color::hex(ridge.color, 255));
                    }
                }
            }
        }
    }
    // A thin veil, so text on top stays easy to read.
    ui.rounded(0.0, 0.0, w, h, 0.0, color::hex(0x000000, 60));
}
