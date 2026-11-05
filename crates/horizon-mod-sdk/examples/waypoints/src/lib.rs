//! Horizon mod: waypoints on the screen. Each one is a rounded square in its
//! colour holding the first letter of its name, with the name above and the
//! distance below, placed where it is in the world and bigger the nearer it
//! is. It shows only while the player looks towards it. Where the player last
//! died a skull is saved. The list, names, colours, sizes and visibility are
//! in Options > Mods > Waypoints; `/wp` works from chat (`/wp help`).
//! Waypoints are kept in the mod's storage.
use horizon_mod_sdk::{
    Mod, Registration, chat, game,
    render::{self, FillShape, ItemShape, Point, Rect, Shape, TextShape},
    storage,
    ui::{self, ButtonWidget, ListWidget, TextWidget, Widget},
};

const MAX: usize = 64;
const PALETTE: [(&str, u32); 9] = [
    ("Red", 0xFF5555),
    ("Orange", 0xFFAA00),
    ("Yellow", 0xFFFF55),
    ("Green", 0x55FF55),
    ("Aqua", 0x55FFFF),
    ("Blue", 0x5599FF),
    ("Purple", 0xAA55FF),
    ("Pink", 0xFF55FF),
    ("White", 0xFFFFFF),
];

const SIZES: [u32; 5] = [50, 75, 100, 150, 200];

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
/// Text request ids.
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
}

struct Waypoints {
    list: Vec<Waypoint>,
    show: bool,
    selected: Option<usize>,
    /// Where the player stood at the last tick with health left.
    last_alive: Option<(i32, i32, i32, String)>,
    alive: bool,
    /// The colour list shows instead of the main page.
    picking: bool,
    /// Where "Add waypoint here" was pressed, while its name is typed.
    pending: Option<(i32, i32, i32, String)>,
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
                "{}|{}|{}|{}|{}|{:06x}|{}|{}|{}",
                w.name, w.x, w.y, w.z, w.dim, w.color, w.visible as u8, w.death as u8, w.size
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode(text: &str) -> Vec<Waypoint> {
    text.lines()
        .filter_map(|line| {
            let p: Vec<&str> = line.split('|').collect();
            if p.len() != 8 && p.len() != 9 {
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
        let _ = storage::set("points", encode(&self.list).as_bytes());
        let _ = storage::set(
            "show",
            if self.show { &b"true"[..] } else { &b"false"[..] },
        );
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

    fn page(&self) {
        if self.picking {
            ui::set_list_colors(
                COLOR_LIST,
                &PALETTE.iter().map(|&(_, c)| c).collect::<Vec<_>>(),
            );
            ui::set_settings(&[
                Widget::Text(TextWidget {
                    id: 101,
                    text: "Pick a colour".into(),
                    color: 0,
                }),
                Widget::List(ListWidget {
                    id: COLOR_LIST,
                    rows: PALETTE.iter().map(|&(n, _)| n.to_owned()).collect(),
                    selected: self
                        .selected
                        .and_then(|i| self.list.get(i))
                        .and_then(|w| PALETTE.iter().position(|&(_, c)| c == w.color))
                        .map(|i| i as u32),
                }),
                Widget::Button(ButtonWidget {
                    id: BACK,
                    label: "Back".into(),
                }),
            ]);
            return;
        }
        let mut widgets = vec![
            Widget::Button(ButtonWidget {
                id: SHOW_ALL,
                label: format!("Waypoints: {}", if self.show { "ON" } else { "OFF" }),
            }),
            Widget::Button(ButtonWidget {
                id: ADD,
                label: "Add waypoint here".into(),
            }),
        ];
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
            &self.list.iter().map(|w| w.color).collect::<Vec<_>>(),
        );
        widgets.push(Widget::List(ListWidget {
            id: LIST,
            rows,
            selected: self.selected.map(|i| i as u32),
        }));
        if let Some(w) = self.selected.and_then(|i| self.list.get(i)) {
            widgets.push(Widget::Button(ButtonWidget {
                id: RENAME,
                label: format!("Rename: {}", w.name),
            }));
            widgets.push(Widget::Button(ButtonWidget {
                id: COLOR,
                label: format!("Colour: {}", color_name(w.color)),
            }));
            widgets.push(Widget::Button(ButtonWidget {
                id: SIZE,
                label: format!("Size: {}%", w.size),
            }));
            widgets.push(Widget::Button(ButtonWidget {
                id: VISIBLE,
                label: format!("Shown: {}", if w.visible { "yes" } else { "no" }),
            }));
            widgets.push(Widget::Button(ButtonWidget {
                id: DELETE,
                label: "Delete this waypoint".into(),
            }));
        }
        ui::set_settings(&widgets);
    }

    /// A rounded square of `half` × 2 on a side centred on (`x`, `y`).
    fn rounded(shapes: &mut Vec<Shape>, x: f32, y: f32, half: f32, color: u32) {
        let r = if half >= 10.0 { 2.0 } else { 1.0 };
        let mut fill = |rx: f32, ry: f32, w: f32, h: f32| {
            shapes.push(Shape::Fill(FillShape {
                rect: Rect { x: rx, y: ry, w, h },
                color,
            }));
        };
        fill(x - half + r, y - half, 2.0 * half - 2.0 * r, 2.0 * half);
        fill(x - half, y - half + r, 2.0 * half, 2.0 * half - 2.0 * r);
        if r > 1.0 {
            fill(
                x - half + 1.0,
                y - half + 1.0,
                2.0 * half - 2.0,
                2.0 * half - 2.0,
            );
        }
    }

    fn draw(&self) {
        if !self.show {
            render::set_hud(&[]);
            return;
        }
        let (Some(view), Some(player)) = (game::get_view(), game::get_player()) else {
            render::set_hud(&[]);
            return;
        };
        let size = render::gui_size();
        let mut shapes = Vec::new();
        for w in self
            .list
            .iter()
            .filter(|w| w.visible && w.dim == player.dimension)
        {
            let at = [
                f64::from(w.x) + 0.5,
                f64::from(w.y) + 0.5,
                f64::from(w.z) + 0.5,
            ];
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
            let scale = w.size as f32 / 100.0;
            let side = (30.0 / (1.0 + dist as f32 / 80.0)).clamp(12.0, 26.0) * scale;
            let half = (side / 2.0).max(5.0);
            if w.death {
                let sc = side / 12.0;
                shapes.push(Shape::Item(ItemShape {
                    item: "minecraft:skeleton_skull".into(),
                    at: Point {
                        x: x - 8.0 * sc,
                        y: y - 8.0 * sc,
                    },
                    scale: sc,
                }));
            } else {
                Self::rounded(&mut shapes, x, y, half + 1.5, 0xE000_0000);
                Self::rounded(&mut shapes, x, y, half, 0xFF00_0000 | w.color);
                let letter: String = w
                    .name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect())
                    .unwrap_or_default();
                shapes.push(Shape::Text(TextShape {
                    text: letter,
                    at: Point { x, y: y - 4.0 },
                    color: 0xFFFFFF,
                    centered: true,
                    shadow: true,
                }));
            }
            shapes.push(Shape::Text(TextShape {
                text: w.name.clone(),
                at: Point {
                    x,
                    y: y - half - 13.0,
                },
                color: w.color,
                centered: true,
                shadow: true,
            }));
            shapes.push(Shape::Text(TextShape {
                text: format!("{}m", dist.round() as i64),
                at: Point {
                    x,
                    y: y + half + 4.0,
                },
                color: 0xDDDDDD,
                centered: true,
                shadow: true,
            }));
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

const HELP: &str = "/wp add [name] | list | del N | name N new name | color N red or #ff8800 | size N 50-300 | hide N | show N | view on or off";

impl Mod for Waypoints {
    fn init() -> Result<(Self, Registration), String> {
        let list = stored("points").map(|t| decode(&t)).unwrap_or_default();
        let show = stored("show").is_none_or(|v| v.trim() != "false");
        let me = Waypoints {
            list,
            show,
            selected: None,
            last_alive: None,
            alive: false,
            picking: false,
            pending: None,
        };
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
                });
                self.list.truncate(MAX);
                self.selected = None;
                self.changed();
                chat::show(&format!("Death point saved at {x} {y} {z}"));
            }
        }
    }

    fn frame(&mut self, _seconds: f32) {
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
                        "{}. {} ({} {} {}) {} {}%{}",
                        i + 1,
                        w.name,
                        w.x,
                        w.y,
                        w.z,
                        color_name(w.color),
                        w.size,
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
            "size" => {
                let (n, v) = rest.split_once(' ').unwrap_or((rest, ""));
                let i = self.index(n)?;
                let size: u32 = v
                    .trim()
                    .trim_end_matches('%')
                    .parse()
                    .ok()
                    .filter(|v| (25..=300).contains(v))
                    .ok_or("Use /wp size N 25 to 300 (100 is normal)")?;
                self.list[i].size = size;
                self.changed();
                Ok(Some(format!("Size is {size}%")))
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
            COLOR_LIST => {
                if let (Some(w), Some(&(_, c))) = (
                    self.selected.and_then(|i| self.list.get_mut(i)),
                    PALETTE.get(row as usize),
                ) {
                    w.color = c;
                }
                self.picking = false;
            }
            BACK => {
                self.picking = false;
                self.page();
                return;
            }
            SIZE => {
                if let Some(w) = self.selected.and_then(|i| self.list.get_mut(i)) {
                    let next = SIZES.iter().position(|&s| s == w.size).map_or(0, |i| {
                        (i + 1) % SIZES.len()
                    });
                    w.size = SIZES[next];
                }
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
            }
            _ => return,
        }
        self.changed();
    }

    fn ui_settings_opened(&mut self) {
        self.picking = false;
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Waypoints);
