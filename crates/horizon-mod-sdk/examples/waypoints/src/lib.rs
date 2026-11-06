//! Horizon mod: waypoints on the screen. Each one is a rounded square in its
//! colour holding the first letter of its name, with the name above and the
//! distance below, placed where it is in the world and bigger the nearer it
//! is. It shows only while the player looks towards it. Where the player last
//! died a skull is saved. The list, names, colours, sizes and visibility are
//! in Options > Mods > Waypoints; `/wp` works from chat (`/wp help`).
//! Waypoints are kept in the mod's storage.
use horizon_mod_sdk::{
    Mod, Registration, chat, game,
    input::{self, Event},
    render::{self, FillShape, ItemShape, Point, Rect, Shape, TextShape},
    storage,
    ui::{self, ButtonWidget, ListWidget, TextWidget, Widget},
};

const MAX: usize = 64;
const PALETTE: [(&str, u32); 29] = [
    ("Red", 0xFF5555),
    ("Orange", 0xFFAA00),
    ("Yellow", 0xFFFF55),
    ("Green", 0x55FF55),
    ("Aqua", 0x55FFFF),
    ("Blue", 0x5599FF),
    ("Purple", 0xAA55FF),
    ("Pink", 0xFF55FF),
    ("White", 0xFFFFFF),
    ("Black", 0x111111),
    ("Dark Gray", 0x555555),
    ("Gray", 0xAAAAAA),
    ("Light Gray", 0xD0D0D0),
    ("Dark Red", 0xAA0000),
    ("Maroon", 0x800000),
    ("Gold", 0xFFD700),
    ("Lime", 0xAAFF00),
    ("Dark Green", 0x00AA00),
    ("Mint", 0x55FFAA),
    ("Teal", 0x00AAAA),
    ("Sky Blue", 0x55AAFF),
    ("Navy", 0x0000AA),
    ("Indigo", 0x5500AA),
    ("Violet", 0xAA00AA),
    ("Magenta", 0xFF00FF),
    ("Hot Pink", 0xFF55AA),
    ("Brown", 0x8B5A2B),
    ("Tan", 0xD2B48C),
    ("Peach", 0xFFB07C),
];

/// A colour as the picker's rows show it: dark ones are lifted so the name
/// can be read on the menu's dark background.
fn lifted(color: u32) -> u32 {
    let lift = |c: u32| c.max(0x77);
    (lift(color >> 16 & 0xFF) << 16) | (lift(color >> 8 & 0xFF) << 8) | lift(color & 0xFF)
}

const SHOW_ALL: u32 = 1;
const LIST: u32 = 2;
const COLOR: u32 = 3;
const VISIBLE: u32 = 4;
const DELETE: u32 = 5;
const ADD: u32 = 6;
const RENAME: u32 = 7;
const SIZE: u32 = 8;
const COLOR_LIST: u32 = 9;
const BACK: u32 = 10;
const OPACITY: u32 = 11;
const EDIT: u32 = 12;
const TEXT_SIZE: u32 = 13;
const TEXT_OPACITY: u32 = 14;
const STYLE_BTN: u32 = 15;
/// Text request ids.
/// The key that opens the waypoints screen.
const KEY: &str = "open";
const ASK_NAME: u32 = 1;
const ASK_NEW: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
struct Waypoint {
    name: String,
    x: i32,
    y: i32,
    z: i32,
    dim: String,
    color: u32,
    visible: bool,
    death: bool,
    /// Percent of the normal size.
    size: u32,
    /// Percent of full strength (how solid the square is).
    opacity: u32,
}

/// How every waypoint is drawn (only the colour is a waypoint's own).
#[derive(Clone, Copy)]
struct Style {
    /// Percent of the normal size of the markers.
    size: u32,
    /// Percent of full strength of the markers.
    opacity: u32,
    /// The same for the distance under a marker.
    text_size: u32,
    text_opacity: u32,
}

impl Style {
    const NORMAL: Style = Style {
        size: 100,
        opacity: 100,
        text_size: 100,
        text_opacity: 100,
    };

    fn encode(self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.size, self.opacity, self.text_size, self.text_opacity
        )
    }

    fn decode(text: &str) -> Style {
        let v: Vec<u32> = text.trim().split('|').filter_map(|p| p.parse().ok()).collect();
        let pick = |i: usize, lo: u32, hi: u32| {
            v.get(i).copied().filter(|n| (lo..=hi).contains(n)).unwrap_or(100)
        };
        Style {
            size: pick(0, 25, 300),
            opacity: pick(1, 10, 100),
            text_size: pick(2, 25, 300),
            text_opacity: pick(3, 10, 100),
        }
    }
}

struct Waypoints {
    list: Vec<Waypoint>,
    style: Style,
    /// The page of the settings every waypoint shares shows.
    style_page: bool,
    show: bool,
    selected: Option<usize>,
    /// Where the player stood at the last tick with health left.
    last_alive: Option<(i32, i32, i32, String)>,
    alive: bool,
    /// The colour list shows instead of the main page.
    picking: bool,
    /// Where "Add waypoint here" was pressed, while its name is typed.
    pending: Option<(i32, i32, i32, String)>,
    /// The world the list belongs to (`game.get-world`); each world keeps
    /// its own waypoints.
    world: Option<String>,
    /// The selected waypoint's page (name, colour, size, opacity) shows.
    editing: bool,
    /// Changed by a slider and not yet written to the storage.
    dirty: bool,
    /// The waypoints screen (opened with the key) is showing.
    screen_open: bool,
}

/// The storage key of a world's waypoints: keys are 64 characters of
/// `[a-z0-9_.-]`, a world's id is not, so it is hashed (FNV-1a).
fn world_key(world: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in world.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("w_{h:016x}")
}

fn clean(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c == '|' || c.is_control() { ' ' } else { c })
        .collect();
    s.trim().chars().take(24).collect()
}

fn encode(list: &[Waypoint]) -> String {
    list.iter()
        .map(|w| {
            format!(
                "{}|{}|{}|{}|{}|{:06x}|{}|{}|{}|{}",
                w.name,
                w.x,
                w.y,
                w.z,
                w.dim,
                w.color,
                w.visible as u8,
                w.death as u8,
                w.size,
                w.opacity
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode(text: &str) -> Vec<Waypoint> {
    text.lines()
        .filter_map(|line| {
            let p: Vec<&str> = line.split('|').collect();
            if !(8..=10).contains(&p.len()) {
                return None;
            }
            Some(Waypoint {
                name: clean(p[0]),
                x: p[1].parse().ok()?,
                y: p[2].parse().ok()?,
                z: p[3].parse().ok()?,
                dim: p[4].to_owned(),
                color: u32::from_str_radix(p[5], 16).ok()? & 0xFF_FFFF,
                visible: p[6] == "1",
                death: p[7] == "1",
                size: p
                    .get(8)
                    .and_then(|v| v.parse().ok())
                    .filter(|v| (25..=300).contains(v))
                    .unwrap_or(100),
                opacity: p
                    .get(9)
                    .and_then(|v| v.parse().ok())
                    .filter(|v| (10..=100).contains(v))
                    .unwrap_or(100),
            })
        })
        .take(MAX)
        .collect()
}

fn parse_color(text: &str) -> Option<u32> {
    let t = text.trim();
    if let Some(hex) = t.strip_prefix('#') {
        return (hex.len() == 6)
            .then(|| u32::from_str_radix(hex, 16).ok())
            .flatten();
    }
    PALETTE
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(t))
        .map(|&(_, c)| c)
}

fn color_name(color: u32) -> String {
    PALETTE
        .iter()
        .find(|&&(_, c)| c == color)
        .map_or_else(|| format!("#{color:06x}"), |&(n, _)| n.to_owned())
}

/// Where a world point lands on a `w` × `h` screen seen from the camera: the
/// pixel, whether it is really in view, and its distance. A point beside or
/// behind the camera is pinned to the screen's edge.
fn project(
    cam: [f64; 3],
    yaw: f32,
    pitch: f32,
    fov: f32,
    (w, h): (f32, f32),
    target: [f64; 3],
) -> ([f32; 2], bool, f64) {
    let d = [target[0] - cam[0], target[1] - cam[1], target[2] - cam[2]];
    let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    let f = [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()];
    let r = [-y.cos(), 0.0, -y.sin()];
    let u = [-y.sin() * p.sin(), p.cos(), y.cos() * p.sin()];
    let dot = |a: [f64; 3]| a[0] * d[0] + a[1] * d[1] + a[2] * d[2];
    let (depth, px, py) = (dot(f), dot(r), dot(u));
    let t = (f64::from(fov).to_radians() / 2.0).tan().max(0.05);
    let aspect = f64::from(w) / f64::from(h.max(1.0));
    let (mut nx, mut ny, mut inside) = if depth > 0.05 {
        (px / (depth * t * aspect), py / (depth * t), true)
    } else {
        let side = if px >= 0.0 { 3.0 } else { -3.0 };
        (side, (py / dist.max(1.0)).clamp(-1.0, 1.0), false)
    };
    if nx.abs() > 1.0 || ny.abs() > 1.0 {
        inside = false;
    }
    nx = nx.clamp(-1.0, 1.0);
    ny = ny.clamp(-1.0, 1.0);
    let (mx, top, bottom) = (16.0, 30.0, 30.0);
    let sx = mx + (w - 2.0 * mx) * (0.5 + 0.5 * nx as f32);
    let sy = top + (h - top - bottom) * (0.5 - 0.5 * ny as f32);
    ([sx, sy], inside, dist)
}

fn stored(key: &str) -> Option<String> {
    storage::get(key).and_then(|v| String::from_utf8(v).ok())
}

impl Waypoints {
    fn save(&self) {
        let _ = storage::set(
            "show",
            if self.show { &b"true"[..] } else { &b"false"[..] },
        );
        let _ = storage::set("style", self.style.encode().as_bytes());
        let Some(world) = &self.world else {
            return;
        };
        if storage::set(&world_key(world), encode(&self.list).as_bytes()).is_err() {
            chat::show("Waypoints could not be saved (too many worlds, or no room)");
        }
    }

    /// Loads the list of the world the player is in when it changed: each
    /// world has its own waypoints, a new world starts with none.
    fn sync_world(&mut self) {
        let now = game::get_world();
        if now == self.world {
            return;
        }
        if self.dirty {
            self.save();
            self.dirty = false;
        }
        self.world = now;
        self.selected = None;
        self.editing = false;
        self.picking = false;
        self.style_page = false;
        self.pending = None;
        self.last_alive = None;
        self.alive = false;
        self.list = match &self.world {
            Some(world) => match stored(&world_key(world)) {
                Some(text) => decode(&text),
                None => {
                    // Waypoints from before each world had its own go to the
                    // first world opened, once.
                    let old = stored("points").map(|t| decode(&t)).unwrap_or_default();
                    if !old.is_empty() {
                        storage::delete("points");
                    }
                    old
                }
            },
            None => Vec::new(),
        };
        if !self.list.is_empty() {
            self.save();
        }
        self.page();
    }

    fn here(&self) -> Option<(i32, i32, i32, String)> {
        let p = game::get_player()?;
        Some((
            p.x.floor() as i32,
            p.y.floor() as i32,
            p.z.floor() as i32,
            p.dimension,
        ))
    }

    /// Saves a waypoint at `at` (or where the player stands) called `name`.
    fn add_at(
        &mut self,
        at: Option<(i32, i32, i32, String)>,
        name: &str,
    ) -> Result<String, String> {
        if self.list.len() >= MAX {
            return Err(format!("At most {MAX} waypoints"));
        }
        let (x, y, z, dim) = at.or_else(|| self.here()).ok_or("Join a world first")?;
        let name = clean(name);
        let name = if name.is_empty() {
            format!("Point {}", self.list.len() + 1)
        } else {
            name
        };
        let color = PALETTE[self.list.len() % PALETTE.len()].1;
        self.list.push(Waypoint {
            name: name.clone(),
            x,
            y,
            z,
            dim,
            color,
            visible: true,
            death: false,
            size: 100,
            opacity: 100,
        });
        self.selected = Some(self.list.len() - 1);
        self.changed();
        Ok(format!("{name} saved at {x} {y} {z}"))
    }

    fn add(&mut self, name: &str) -> Result<String, String> {
        self.add_at(None, name)
    }

    fn changed(&self) {
        self.save();
        self.page();
    }

    /// Pushes the widgets to the settings page, and to the screen when it is open.
    fn show(&self, widgets: &[Widget]) {
        ui::set_settings(widgets);
        if self.screen_open {
            ui::open_screen(&ui::Screen {
                title: "Waypoints".into(),
                widgets: widgets.to_vec(),
            });
        }
    }

    fn page(&self) {
        let text = |id: u32, t: &str| {
            Widget::Text(TextWidget {
                id,
                text: t.into(),
                color: 0,
            })
        };
        let button = |id: u32, label: String| Widget::Button(ButtonWidget { id, label });
        if self.world.is_none() {
            self.show(&[text(100, "Join a world to see its waypoints")]);
            return;
        }
        if self.picking {
            ui::set_list_colors(
                COLOR_LIST,
                &PALETTE.iter().map(|&(_, c)| lifted(c)).collect::<Vec<_>>(),
            );
            self.show(&[
                text(101, "Pick a colour"),
                Widget::List(ListWidget {
                    id: COLOR_LIST,
                    rows: PALETTE.iter().map(|&(n, _)| n.to_owned()).collect(),
                    selected: self
                        .selected
                        .and_then(|i| self.list.get(i))
                        .and_then(|w| PALETTE.iter().position(|&(_, c)| c == w.color))
                        .map(|i| i as u32),
                }),
                button(BACK, "Back".into()),
            ]);
            return;
        }
        if self.style_page {
            let st = self.style;
            ui::set_slider(SIZE, 25, 300, st.size);
            ui::set_slider(OPACITY, 10, 100, st.opacity);
            ui::set_slider(TEXT_SIZE, 25, 300, st.text_size);
            ui::set_slider(TEXT_OPACITY, 10, 100, st.text_opacity);
            self.show(&[
                button(
                    SHOW_ALL,
                    format!("All waypoints: {}", if self.show { "ON" } else { "OFF" }),
                ),
                button(SIZE, format!("Marker size: {}%", st.size)),
                button(OPACITY, format!("Marker opacity: {}%", st.opacity)),
                button(TEXT_SIZE, format!("Distance size: {}%", st.text_size)),
                button(
                    TEXT_OPACITY,
                    format!("Distance opacity: {}%", st.text_opacity),
                ),
                button(BACK, "Back".into()),
            ]);
            return;
        }
        if self.editing
            && let Some(w) = self.selected.and_then(|i| self.list.get(i))
        {
            self.show(&[
                button(RENAME, format!("Name: {}", w.name)),
                button(COLOR, format!("Colour: {}", color_name(w.color))),
                button(STYLE_BTN, "Look of all waypoints".into()),
                button(BACK, "Back".into()),
            ]);
            return;
        }
        let mut widgets = vec![button(ADD, "Add waypoint".into())];
        let rows: Vec<String> = self
            .list
            .iter()
            .enumerate()
            .map(|(i, w)| {
                format!(
                    "{}. {}{}  ({} {} {})",
                    i + 1,
                    w.name,
                    if w.visible { "" } else { " [hidden]" },
                    w.x,
                    w.y,
                    w.z
                )
            })
            .collect();
        ui::set_list_colors(
            LIST,
            &self.list.iter().map(|w| lifted(w.color)).collect::<Vec<_>>(),
        );
        widgets.push(Widget::List(ListWidget {
            id: LIST,
            rows,
            selected: self.selected.map(|i| i as u32),
        }));
        if let Some(w) = self.selected.and_then(|i| self.list.get(i)) {
            widgets.push(button(EDIT, format!("Edit: {}", w.name)));
            widgets.push(button(
                VISIBLE,
                format!("{}: {}", if w.visible { "Hide" } else { "Show" }, w.name),
            ));
            widgets.push(button(DELETE, format!("Delete: {}", w.name)));
        } else {
            widgets.push(button(STYLE_BTN, "Look of all waypoints".into()));
            widgets.push(text(102, "Tap a waypoint to edit, hide or delete it"));
        }
        self.show(&widgets);
    }

    /// A rectangle of the HUD.
    fn fill(shapes: &mut Vec<Shape>, x: f32, y: f32, w: f32, h: f32, color: u32) {
        shapes.push(Shape::Fill(FillShape {
            rect: Rect { x, y, w, h },
            color,
        }));
    }

    /// `color` moved `t` of the way to `to` (both 0xRRGGBB).
    fn mix(color: u32, to: u32, t: f32) -> u32 {
        let part = |shift: u32| {
            let a = (color >> shift & 0xFF) as f32;
            let b = (to >> shift & 0xFF) as f32;
            ((a + (b - a) * t).round() as u32).min(255) << shift
        };
        part(16) | part(8) | part(0)
    }

    /// Text drawn with a 3 x 5 pixel font in blocks of `unit` GUI pixels (the
    /// game's own font has no size or see-through), centred on `cx`. The
    /// characters it has are 0-9, A-Z, `m`, `k` and `.`; false when one is
    /// missing (nothing is drawn then).
    fn pixel_text(
        shapes: &mut Vec<Shape>,
        text: &str,
        cx: f32,
        top: f32,
        unit: f32,
        rgb: u32,
        alpha: u32,
    ) -> bool {
        let glyphs: Option<Vec<[&str; 5]>> = text.chars().map(glyph).collect();
        let Some(glyphs) = glyphs else {
            return false;
        };
        let cols: usize =
            glyphs.iter().map(|g| g[0].len()).sum::<usize>() + glyphs.len().saturating_sub(1);
        let left = cx - cols as f32 * unit / 2.0;
        // A shadow one block down and right, then the text.
        for (shift, color) in [
            (unit * 0.7, (alpha * 6 / 10) << 24),
            (0.0, alpha << 24 | rgb),
        ] {
            if shift > 0.0 && shapes.len() > 700 {
                continue;
            }
            let mut col = 0usize;
            for g in &glyphs {
                for (r, row) in g.iter().enumerate() {
                    let bits = row.as_bytes();
                    let mut c = 0;
                    while c < bits.len() {
                        if bits[c] == b'1' {
                            let start = c;
                            while c < bits.len() && bits[c] == b'1' {
                                c += 1;
                            }
                            if shapes.len() < 1000 {
                                Self::fill(
                                    shapes,
                                    left + (col + start) as f32 * unit + shift,
                                    top + r as f32 * unit + shift,
                                    (c - start) as f32 * unit,
                                    unit,
                                    color,
                                );
                            }
                        } else {
                            c += 1;
                        }
                    }
                }
                col += g[0].len() + 1;
            }
        }
        true
    }

    /// A waypoint's marker: a square of blocks with a dark edge and a lit top
    /// and left, like a Minecraft button, its letter in the middle.
    fn marker(
        shapes: &mut Vec<Shape>,
        cx: f32,
        cy: f32,
        side: f32,
        rgb: u32,
        alpha: u32,
        letter: char,
    ) {
        let s = side.round().max(8.0);
        let (x, y) = ((cx - s / 2.0).round(), (cy - s / 2.0).round());
        let a = alpha << 24;
        let (light, dark) = (Self::mix(rgb, 0xFFFFFF, 0.4), Self::mix(rgb, 0x000000, 0.4));
        Self::fill(shapes, x + 1.0, y, s - 2.0, 1.0, a);
        Self::fill(shapes, x + 1.0, y + s - 1.0, s - 2.0, 1.0, a);
        Self::fill(shapes, x, y + 1.0, 1.0, s - 2.0, a);
        Self::fill(shapes, x + s - 1.0, y + 1.0, 1.0, s - 2.0, a);
        Self::fill(shapes, x + 1.0, y + 1.0, s - 2.0, 1.0, a | light);
        Self::fill(shapes, x + 1.0, y + 2.0, 1.0, s - 4.0, a | light);
        Self::fill(shapes, x + 1.0, y + s - 2.0, s - 2.0, 1.0, a | dark);
        Self::fill(shapes, x + s - 2.0, y + 2.0, 1.0, s - 4.0, a | dark);
        Self::fill(shapes, x + 2.0, y + 2.0, s - 4.0, s - 4.0, a | rgb);
        // The letter in white, or black on a light colour.
        let bright = (rgb >> 16 & 0xFF) * 3 + (rgb >> 8 & 0xFF) * 6 + (rgb & 0xFF);
        let ink = if bright > 1700 { 0x111111 } else { 0xFFFFFF };
        let unit = s / 9.0;
        let drawn = Self::pixel_text(
            shapes,
            &letter.to_string(),
            cx,
            cy - 2.5 * unit,
            unit,
            ink,
            alpha,
        );
        if !drawn {
            shapes.push(Shape::Text(TextShape {
                text: letter.to_string(),
                at: Point { x: cx, y: cy - 4.0 },
                color: ink,
                centered: true,
                shadow: false,
            }));
        }
    }

    /// Where `w` is as seen from the dimension `dim`: the Nether's blocks are
    /// 8 Overworld blocks, so a waypoint of one shows in the other too.
    fn place(w: &Waypoint, dim: &str) -> Option<[f64; 3]> {
        const OVERWORLD: &str = "minecraft:overworld";
        const NETHER: &str = "minecraft:the_nether";
        let (x, y, z) = (f64::from(w.x) + 0.5, f64::from(w.y) + 0.5, f64::from(w.z) + 0.5);
        if w.dim == dim {
            Some([x, y, z])
        } else if w.dim == OVERWORLD && dim == NETHER {
            Some([x / 8.0, y, z / 8.0])
        } else if w.dim == NETHER && dim == OVERWORLD {
            Some([x * 8.0, y, z * 8.0])
        } else {
            None
        }
    }

    fn draw(&self) {
        if !self.show || self.world.is_none() {
            render::set_hud(&[]);
            return;
        }
        let (Some(view), Some(player)) = (game::get_view(), game::get_player()) else {
            render::set_hud(&[]);
            return;
        };
        let size = render::gui_size();
        let mut shapes = Vec::new();
        for w in self.list.iter().filter(|w| w.visible) {
            let Some(at) = Self::place(w, &player.dimension) else {
                continue;
            };
            let ([x, y], inside, dist) = project(
                [view.x, view.y, view.z],
                view.yaw,
                view.pitch,
                view.fov,
                size,
                at,
            );
            // Only while looking towards it.
            if !inside {
                continue;
            }
            let st = self.style;
            let side = (30.0 / (1.0 + dist as f32 / 80.0)).clamp(12.0, 26.0) * st.size as f32 / 100.0;
            let side = side.max(8.0);
            let alpha = (st.opacity * 255 / 100).min(255);
            let below = if w.death {
                let sc = side / 12.0;
                shapes.push(Shape::Item(ItemShape {
                    item: "minecraft:skeleton_skull".into(),
                    at: Point {
                        x: x - 8.0 * sc,
                        y: y - 8.0 * sc,
                    },
                    scale: sc,
                }));
                8.0 * sc
            } else {
                let letter = w
                    .name
                    .chars()
                    .next()
                    .and_then(|c| c.to_uppercase().next())
                    .unwrap_or('?');
                Self::marker(&mut shapes, x, y, side, w.color, alpha, letter);
                side.round().max(8.0) / 2.0
            };
            // How far, under the marker: metres, or kilometres from 1000.
            let text = if dist < 1000.0 {
                format!("{}m", dist.round() as i64)
            } else if dist < 100_000.0 {
                format!("{:.1}km", dist / 1000.0)
            } else {
                format!("{}km", (dist / 1000.0).round() as i64)
            };
            let unit = 1.4 * st.text_size as f32 / 100.0;
            let text_alpha = (st.text_opacity * 255 / 100).min(255);
            Self::pixel_text(
                &mut shapes,
                &text,
                x,
                y + below + 3.0,
                unit,
                0xDDDDDD,
                text_alpha,
            );
        }
        render::set_hud(&shapes);
    }

    fn index(&self, text: &str) -> Result<usize, String> {
        let n: usize = text
            .trim()
            .parse()
            .map_err(|_| "Give the waypoint's number (see /wp list)".to_owned())?;
        if (1..=self.list.len()).contains(&n) {
            Ok(n - 1)
        } else {
            Err(format!("No waypoint {n} (see /wp list)"))
        }
    }

    /// The names the player typed (Rename, Add).
    fn typed(&mut self) {
        for (id, text) in ui::poll_text() {
            match id {
                ASK_NAME => {
                    let name = clean(&text);
                    if let Some(w) = self.selected.and_then(|i| self.list.get_mut(i))
                        && !name.is_empty()
                    {
                        w.name = name;
                        self.changed();
                    }
                }
                ASK_NEW => {
                    let at = self.pending.take();
                    if let Err(e) = self.add_at(at, &text) {
                        chat::show(&e);
                    }
                }
                _ => {}
            }
        }
    }
}

const HELP: &str = "/wp add [name] | list | del N | name N new name | color N red or #ff8800 | size N 25-300 | opacity N 10-100 | textsize N | textopacity N | hide N | show N | view on or off";

impl Mod for Waypoints {
    fn init() -> Result<(Self, Registration), String> {
        let show = stored("show").is_none_or(|v| v.trim() != "false");
        let mut me = Waypoints {
            style: stored("style").map_or(Style::NORMAL, |t| Style::decode(&t)),
            style_page: false,
            list: Vec::new(),
            show,
            selected: None,
            last_alive: None,
            alive: false,
            picking: false,
            pending: None,
            world: None,
            editing: false,
            dirty: false,
            screen_open: false,
        };
        let _ = input::register_key(KEY, "Open waypoints", "key.keyboard.b");
        me.sync_world();
        me.page();
        Ok((
            me,
            Registration::new()
                .command("wp")
                .command("waypoint")
                .on_tick()
                .on_frame(),
        ))
    }

    fn tick(&mut self, _tick: u64) {
        self.sync_world();
        if self.dirty {
            self.save();
            self.dirty = false;
        }
        for event in input::poll() {
            if matches!(event, Event::KeyDown(ref k) if k == KEY) {
                self.sync_world();
                self.editing = false;
                self.picking = false;
                self.screen_open = true;
                self.page();
            }
        }
        self.typed();
        let Some(p) = game::get_player() else {
            return;
        };
        if p.health > 0.0 {
            self.alive = true;
            self.last_alive = Some((
                p.x.floor() as i32,
                p.y.floor() as i32,
                p.z.floor() as i32,
                p.dimension,
            ));
        } else if self.alive {
            self.alive = false;
            if let Some((x, y, z, dim)) = self.last_alive.clone() {
                self.list.retain(|w| !w.death);
                self.list.push(Waypoint {
                    name: "Death".into(),
                    x,
                    y,
                    z,
                    dim,
                    color: 0xFF5555,
                    visible: true,
                    death: true,
                    size: 100,
                    opacity: 100,
                });
                self.list.truncate(MAX);
                self.selected = None;
                self.changed();
                chat::show(&format!("Death point saved at {x} {y} {z}"));
            }
        }
    }

    fn frame(&mut self, _seconds: f32) {
        self.sync_world();
        self.typed();
        self.draw();
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let args = args.trim();
        let (sub, rest) = args.split_once(' ').unwrap_or((args, ""));
        let rest = rest.trim();
        match sub {
            "" | "help" => Ok(Some(HELP.into())),
            "add" => self.add(rest).map(Some),
            "list" => {
                if self.list.is_empty() {
                    return Ok(Some("No waypoints. /wp add [name]".into()));
                }
                for (i, w) in self.list.iter().enumerate() {
                    chat::show(&format!(
                        "{}. {} ({} {} {}) {}{}",
                        i + 1,
                        w.name,
                        w.x,
                        w.y,
                        w.z,
                        color_name(w.color),
                        if w.visible { "" } else { ", hidden" }
                    ));
                }
                Ok(None)
            }
            "del" => {
                let i = self.index(rest)?;
                let w = self.list.remove(i);
                self.selected = None;
                self.changed();
                Ok(Some(format!("Deleted {}", w.name)))
            }
            "name" => {
                let (n, name) = rest.split_once(' ').unwrap_or((rest, ""));
                let i = self.index(n)?;
                let name = clean(name);
                if name.is_empty() {
                    return Err("Use /wp name N new name".into());
                }
                self.list[i].name = name.clone();
                self.changed();
                Ok(Some(format!("Renamed to {name}")))
            }
            "color" | "colour" => {
                let (n, c) = rest.split_once(' ').unwrap_or((rest, ""));
                let i = self.index(n)?;
                let color = parse_color(c)
                    .ok_or("Use a colour name (red, orange, yellow, green, aqua, blue, purple, pink, white) or #rrggbb")?;
                self.list[i].color = color;
                self.changed();
                Ok(Some(format!("Colour is {}", color_name(color))))
            }
            "size" | "opacity" | "textsize" | "textopacity" => {
                let (lo, hi) = if sub.ends_with("size") { (25, 300) } else { (10, 100) };
                let n: u32 = rest
                    .trim()
                    .trim_end_matches('%')
                    .parse()
                    .ok()
                    .filter(|v| (lo..=hi).contains(v))
                    .ok_or(format!("Use /wp {sub} {lo} to {hi} (for all waypoints)"))?;
                match sub {
                    "size" => self.style.size = n,
                    "opacity" => self.style.opacity = n,
                    "textsize" => self.style.text_size = n,
                    _ => self.style.text_opacity = n,
                }
                self.changed();
                Ok(Some(format!("{sub} is {n}%")))
            }
            "hide" | "show" => {
                let i = self.index(rest)?;
                self.list[i].visible = sub == "show";
                self.changed();
                Ok(Some(format!(
                    "{} is {}",
                    self.list[i].name,
                    if sub == "show" { "shown" } else { "hidden" }
                )))
            }
            "view" => {
                self.show = rest != "off";
                self.changed();
                Ok(Some(format!(
                    "Waypoints {}",
                    if self.show { "on" } else { "off" }
                )))
            }
            _ => Err(HELP.into()),
        }
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        match id {
            SHOW_ALL => self.show = !self.show,
            ADD => {
                self.pending = self.here();
                if self.pending.is_none() {
                    chat::show("Join a world first");
                    return;
                }
                ui::request_text(ASK_NEW, "Name of the new waypoint", "");
                return;
            }
            LIST => {
                self.selected = Some(row as usize).filter(|&i| i < self.list.len());
                self.page();
                return;
            }
            RENAME => {
                if let Some(w) = self.selected.and_then(|i| self.list.get(i)) {
                    ui::request_text(ASK_NAME, "New name", &w.name);
                }
                return;
            }
            COLOR => {
                self.picking = true;
                self.page();
                return;
            }
            EDIT => {
                self.editing = self.selected.is_some();
                self.page();
                return;
            }
            COLOR_LIST => {
                if let (Some(w), Some(&(_, c))) = (
                    self.selected.and_then(|i| self.list.get_mut(i)),
                    PALETTE.get(row as usize),
                ) {
                    w.color = c;
                }
                self.picking = false;
            }
            STYLE_BTN => {
                self.style_page = true;
                self.page();
                return;
            }
            BACK => {
                if self.picking {
                    self.picking = false;
                } else if self.style_page {
                    self.style_page = false;
                } else {
                    self.editing = false;
                }
                self.page();
                return;
            }
            // The sliders: `row` is the value dragged to.
            SIZE | OPACITY | TEXT_SIZE | TEXT_OPACITY => {
                let stepped = ((row + 2) / 5 * 5).clamp(25, 300);
                let strength = row.clamp(10, 100);
                match id {
                    SIZE => self.style.size = stepped,
                    OPACITY => self.style.opacity = strength,
                    TEXT_SIZE => self.style.text_size = stepped,
                    _ => self.style.text_opacity = strength,
                }
                self.dirty = true;
                self.page();
                return;
            }
            VISIBLE => {
                if let Some(w) = self.selected.and_then(|i| self.list.get_mut(i)) {
                    w.visible = !w.visible;
                }
            }
            DELETE => {
                if let Some(i) = self.selected.filter(|&i| i < self.list.len()) {
                    self.list.remove(i);
                }
                self.selected = None;
                self.editing = false;
            }
            _ => return,
        }
        self.changed();
    }

    fn ui_closed(&mut self) {
        self.screen_open = false;
    }

    fn ui_settings_opened(&mut self) {
        self.sync_world();
        self.picking = false;
        self.editing = false;
        self.style_page = false;
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Waypoints);

/// The 3 x 5 (or wider) pixel font of the markers: rows of `1` and `0`.
fn glyph(c: char) -> Option<[&'static str; 5]> {
    Some(match c {
        '0' => ["111", "101", "101", "101", "111"],
        '1' => ["010", "110", "010", "010", "111"],
        '2' => ["111", "001", "111", "100", "111"],
        '3' => ["111", "001", "111", "001", "111"],
        '4' => ["101", "101", "111", "001", "001"],
        '5' => ["111", "100", "111", "001", "111"],
        '6' => ["111", "100", "111", "101", "111"],
        '7' => ["111", "001", "001", "010", "010"],
        '8' => ["111", "101", "111", "101", "111"],
        '9' => ["111", "101", "111", "001", "111"],
        '.' => ["0", "0", "0", "0", "1"],
        'm' => ["00000", "11010", "10101", "10101", "10101"],
        'k' => ["101", "101", "110", "101", "101"],
        'A' => ["010", "101", "111", "101", "101"],
        'B' => ["110", "101", "110", "101", "110"],
        'C' => ["011", "100", "100", "100", "011"],
        'D' => ["110", "101", "101", "101", "110"],
        'E' => ["111", "100", "110", "100", "111"],
        'F' => ["111", "100", "110", "100", "100"],
        'G' => ["011", "100", "101", "101", "011"],
        'H' => ["101", "101", "111", "101", "101"],
        'I' => ["111", "010", "010", "010", "111"],
        'J' => ["001", "001", "001", "101", "010"],
        'K' => ["101", "101", "110", "101", "101"],
        'L' => ["100", "100", "100", "100", "111"],
        'M' => ["10001", "11011", "10101", "10001", "10001"],
        'N' => ["1001", "1101", "1011", "1001", "1001"],
        'O' => ["010", "101", "101", "101", "010"],
        'P' => ["110", "101", "110", "100", "100"],
        'Q' => ["010", "101", "101", "110", "011"],
        'R' => ["110", "101", "110", "101", "101"],
        'S' => ["011", "100", "010", "001", "110"],
        'T' => ["111", "010", "010", "010", "010"],
        'U' => ["101", "101", "101", "101", "111"],
        'V' => ["101", "101", "101", "101", "010"],
        'W' => ["10001", "10001", "10101", "11011", "10001"],
        'X' => ["101", "101", "010", "101", "101"],
        'Y' => ["101", "101", "010", "010", "010"],
        'Z' => ["111", "001", "010", "100", "111"],
        _ => return None,
    })
}
