use super::*;
use horizon_client::{osk::Output, ui_theme::color};
use winit::{
    event::{KeyEvent, Touch},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    HomeMenu,
    Page(Page),
    Play,
    Search,
    #[allow(dead_code)]
    ShaderSource(crate::shader_catalog::Origin),
    #[allow(dead_code)]
    ModSource(crate::shader_catalog::Origin),
    Field(&'static str),
    FinishNameEdit,
    /// Paste the clipboard text into this field.
    PasteInto(&'static str),
    Install(usize),
    Import(horizon_client::content_files::Kind),
    ToggleSection(usize),
    ToggleLocal(usize),
    DeleteLocal(usize),
    /// The dots of a built-in mod: it cannot be removed.
    BuiltInInfo,
    ConfirmDeleteLocal,
    ConfirmExit,
    Next,
    Previous,
    Profile,
    #[allow(dead_code)]
    Logs,
    RecordPerformance,
    ExportPerformance,
    ExportDiagnostics,
    Account,
    SignIn,
    ElySignIn,
    ResetSettings,
    MicrosoftAccount,
    OfflineAccount,
    SubmitEly,
    SavedAccounts,
    SavedPage(bool),
    SelectAccount(usize),
    SelectOffline,
    WardrobeKind(bool),
    WardrobeSelect(usize),
    WardrobeImport,
    WardrobeCloud,
    WardrobeUpload,
    WardrobeActivateCape(usize),
    WardrobeUseCloud,
    WardrobeModel,
    WardrobeRotate(i8),
    WardrobeZoom(bool),
    WardrobeHideCape,
    DeviceSignIn,
    CopyLoginCode,
    CancelLogin,
    ReviewApp,
    Repository,
    SignOut,
    LoginLink,
    CloseLogin,
    Renderer,
    RenderScale,
    ChooseRenderScale(crate::display::RenderScale),
    Orientation,
    InputHost,
    ChooseInputHost(crate::catalog::InputHost),
    ChooseOrientation(crate::catalog::Orientation),
    ChooseRenderer(crate::catalog::RendererMode),
    ChooseProfile(u32),
    Dismiss,
    Immersive,
    Cutout,
    Aspect,
    ChooseAspect(crate::display::Aspect),
    DisplayHz,
    ChooseDisplayHz(u16),
    CacheBudget,
    ChooseCacheBudget(u32),
    ToggleBundled(&'static str),
    WorldFiles,
    /// Pick a world ZIP and unpack it into the saves folder.
    ImportWorld,
    GameFiles,
    WorkerLimit,
    ChooseWorkerLimit(u32),
    PerformanceHints,
    GpuProfile,
}

/// A cheap, stable number from a cell's place, for the stone texture and the cracks.
fn cell_hash(x: i32, y: i32, seed: i32) -> u32 {
    let mut n = (x.wrapping_mul(73_856_093)
        ^ y.wrapping_mul(19_349_663)
        ^ seed.wrapping_mul(83_492_791)) as u32;
    n ^= n >> 13;
    n = n.wrapping_mul(0x5bd1_e995);
    n ^ (n >> 15)
}

/// The launcher's look. `true`: soft glass plates with rounded corners and the plain font;
/// `false`: the Minecraft block plates (kept as the git tag `minecraft-style-launcher`).
pub(super) const GLASS: bool = true;

/// A glass plate: a soft shadow, a faint light rim, a translucent body (white-tinted when `base`
/// is the neutral grey, the colour itself when it is an accent) and a gloss across the top.
fn glass_plate(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32, base: u32, big: bool) {
    let radius = if big {
        (h * 0.22).clamp(14.0, 26.0)
    } else {
        (h * 0.3).clamp(8.0, 18.0)
    };
    let neutral = base == 0x5a5a5a;
    // Clear glass: what is behind shows through. A faint tint, a one-pixel rim in the top bar's
    // dark grey, and one thin bright line along the top edge where the light catches.
    let body = if neutral {
        color::hex(0xffffff, 26)
    } else {
        color::hex(base, 112)
    };
    ui.rounded(x, y, w, h, radius, body);
}

/// A Minecraft-style block plate: a black outline, a speckled face, a light top and left edge and
/// a dark bottom and right edge, so it reads as a raised block. `base` is 0xRRGGBB.
fn block_plate(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32, base: u32, cracks: bool) {
    if GLASS {
        glass_plate(ui, x, y, w, h, base, cracks);
        return;
    }
    let edge = 2.0;
    ui.rounded(x, y, w, h, 0.0, color::hex(0x000000, 255));
    let (ix, iy, iw, ih) = (x + edge, y + edge, w - 2.0 * edge, h - 2.0 * edge);
    ui.rounded(ix, iy, iw, ih, 0.0, color::hex(base, 255));
    // Speckles: some cells a little lighter or darker, like the game's stone.
    let cell = 6.0;
    let (cols, rows) = ((iw / cell).ceil() as i32, (ih / cell).ceil() as i32);
    for cy in 0..rows {
        for cx in 0..cols {
            let n = cell_hash(cx, cy, base as i32);
            if n % 3 != 0 {
                continue;
            }
            let (px, py) = (ix + cx as f32 * cell, iy + cy as f32 * cell);
            let (pw, ph) = (cell.min(ix + iw - px), cell.min(iy + ih - py));
            let tint = if n % 2 == 0 {
                color::hex(0xffffff, 16)
            } else {
                color::hex(0x000000, 28)
            };
            ui.rounded(px, py, pw, ph, 0.0, tint);
        }
    }
    if cracks && iw >= 24.0 && ih >= 14.0 {
        // One thin, uneven crack wandering down from the top, a short side branch,
        // and a few small chips: worn stone, not a pattern.
        let mut cx = ix + iw * 0.58;
        let mut cy = iy;
        let reach = iy + ih * 0.7;
        let mut k = 0;
        while cy < reach {
            let n = cell_hash(k, 11, 5);
            let step = 4.0 + (n % 6) as f32;
            let jog = match n % 4 {
                0 => -3.0,
                1 => 2.0,
                2 => 4.0,
                _ => -1.0,
            };
            let len = step.min(reach - cy).max(1.0);
            ui.rounded(cx, cy, 2.0, len, 0.0, color::hex(0x000000, 150));
            ui.rounded(cx + 2.0, cy, 1.0, len, 0.0, color::hex(0xffffff, 26));
            if k == 3 {
                // The branch: a short stub heading sideways and down.
                ui.rounded(
                    cx + 2.0,
                    cy + len - 2.0,
                    9.0,
                    2.0,
                    0.0,
                    color::hex(0x000000, 130),
                );
                ui.rounded(
                    cx + 10.0,
                    cy + len,
                    2.0,
                    5.0,
                    0.0,
                    color::hex(0x000000, 110),
                );
            }
            cy += len;
            cx = (cx + jog).clamp(ix + 4.0, (ix + iw - 8.0).max(ix + 4.0));
            k += 1;
        }
        for (fx, fy, size) in [(0.12, 0.62, 4.0), (0.81, 0.3, 3.0), (0.33, 0.2, 3.0)] {
            ui.rounded(
                ix + iw * fx,
                iy + ih * fy,
                size,
                size,
                0.0,
                color::hex(0x000000, 70),
            );
        }
    }
    let bevel = 3.0;
    ui.rounded(ix, iy, iw, bevel, 0.0, color::hex(0xffffff, 70));
    ui.rounded(ix, iy, bevel, ih, 0.0, color::hex(0xffffff, 50));
    ui.rounded(
        ix,
        iy + ih - bevel,
        iw,
        bevel,
        0.0,
        color::hex(0x000000, 95),
    );
    ui.rounded(
        ix + iw - bevel,
        iy,
        bevel,
        ih,
        0.0,
        color::hex(0x000000, 80),
    );
}

/// A white gear made of squares, centred on a square button.
/// A small framed tile for a mod without a picture: its colour comes from its name, its first letter in the middle.
fn mod_tile(ui: &mut Ui, x: f32, y: f32, name: &str) {
    let hash = name
        .bytes()
        .fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let tint = [0x6a4cc4, 0x3f8f6b, 0xb5683a, 0x3a7fb5, 0xa84a7a][(hash % 5) as usize];
    ui.rounded(x, y + 2.0, 60.0, 60.0, 0.0, color::hex(0xffffff, 255));
    ui.rounded(x + 1.0, y + 3.0, 58.0, 58.0, 0.0, color::hex(tint, 255));
    let letter = name
        .chars()
        .next()
        .map_or(String::new(), |c| c.to_uppercase().to_string());
    let width = ui.measure(Face::Medium, 30.0, &letter);
    ui.text(
        Face::Medium,
        30.0,
        x + (60.0 - width) / 2.0,
        y + 17.0,
        color::hex(0xffffff, 255),
        &letter,
    );
}

fn gear_icon(ui: &mut Ui, x: f32, y: f32, size: f32) {
    const GEAR: [&str; 11] = [
        "....###....",
        "..#.###.#..",
        ".#########.",
        "..##...##..",
        "###.....###",
        "###.....###",
        "###.....###",
        "..##...##..",
        ".#########.",
        "..#.###.#..",
        "....###....",
    ];
    // A little smaller than the face next to it.
    let cell = (size * 0.9 / 11.0).floor().max(2.0);
    let left = x + (size - cell * 11.0) / 2.0;
    let top = y + (size - cell * 11.0) / 2.0;
    for (row, line) in GEAR.iter().enumerate() {
        for (col, c) in line.bytes().enumerate() {
            if c == b'#' {
                let (px, py) = (left + col as f32 * cell, top + row as f32 * cell);
                ui.rounded(
                    px + 1.0,
                    py + 1.0,
                    cell,
                    cell,
                    0.0,
                    color::hex(0x252525, 255),
                );
                ui.rounded(px, py, cell, cell, 0.0, color::hex(0xa4a5ad, 255));
            }
        }
    }
}

/// Three small white dots, with no plate: the "more" button of a mod row.
fn more_dots(ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>, rect: [f32; 4], action: Action) {
    let [x, y, w, h] = rect;
    let dot = 5.0;
    let gap = 5.0;
    let left = x + (w - (3.0 * dot + 2.0 * gap)) / 2.0;
    let top = y + (h - dot) / 2.0;
    for i in 0..3 {
        let dx = left + i as f32 * (dot + gap);
        ui.rounded(
            dx + 1.0,
            top + 1.0,
            dot,
            dot,
            0.0,
            color::hex(0x252525, 255),
        );
        ui.rounded(dx, top, dot, dot, 0.0, color::hex(0xffffff, 255));
    }
    targets.push((rect, action));
}

/// The drawn plate inside a button's touch rectangle: a little smaller, centred up and down,
/// so the buttons do not crowd the screen (the whole rectangle still takes the touch).
fn plate_rect(rect: [f32; 4], full: bool) -> (f32, f32, f32, f32) {
    let [x, y, w, h] = rect;
    if full {
        return (x, y, w, h);
    }
    let ph = (h * 0.86).clamp(30.0_f32.min(h), h);
    (x, y + (h - ph) / 2.0, w, ph)
}

/// White text with the game's dark drop shadow.
fn shadow_text(ui: &mut Ui, face: Face, size: f32, x: f32, y: f32, text: &str) {
    if GLASS {
        ui.text(
            face,
            size,
            x + 1.0,
            y + 1.0,
            color::hex(0x000000, 110),
            text,
        );
        ui.text(face, size, x, y, color::hex(0xffffff, 255), text);
        return;
    }
    ui.text(
        face,
        size,
        x + 1.5,
        y + 1.5,
        color::hex(0x252525, 255),
        text,
    );
    ui.text(face, size, x, y, color::hex(0xffffff, 255), text);
}

/// The Add-ons sections as small block buttons, drawn in the row under the tabs.
fn section_chips(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    w: f32,
    h: f32,
    right_reserved: f32,
    current: Page,
) {
    // Under the sections: a thin white rule, and a slightly darker ground for what belongs to them.
    let wide = w >= 700.0;
    let dy = if wide { 20.0 } else { 0.0 };
    if !wide {
        ui.rounded(
            0.0,
            169.0 + dy,
            w,
            (h - 197.0 - dy).max(0.0),
            0.0,
            color::hex(0x0f1013, 255),
        );
        ui.rounded(
            16.0,
            168.0 + dy,
            w - 32.0,
            0.8,
            0.0,
            color::hex(0xb9bac2, 235),
        );
    }
    let space = (w - 64.0 - right_reserved - 8.0).max(120.0);
    let width = ((space - 24.0) / 5.0).clamp(44.0, if wide { 100.0 } else { 96.0 });
    let pages: Vec<(&str, Page)> = if wide {
        vec![
            ("Mods", Page::Mods),
            ("Packs", Page::Packs),
            ("Skins", Page::Skins),
            ("Files", Page::Installed),
        ]
    } else {
        vec![
            ("Mods", Page::Mods),
            ("Shaders", Page::Shaders),
            ("Packs", Page::Packs),
            ("Files", Page::Installed),
        ]
    };
    for (i, label, action, chosen) in pages
        .into_iter()
        .map(|(label, page)| (label, Action::Page(page), page == current))
        .chain(if wide {
            None
        } else {
            Some(("Worlds", Action::ImportWorld, false))
        })
        .enumerate()
        .map(|(i, (label, action, chosen))| (i, label, action, chosen))
    {
        button_as(
            ui,
            targets,
            if wide {
                [32.0 + i as f32 * (width + 8.0), 78.0, width, 36.0]
            } else {
                [32.0 + i as f32 * (width + 6.0), 134.0, width, 34.0]
            },
            label,
            action,
            chosen,
            Some(SECTION),
        );
    }
}

pub(super) fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
) {
    button_as(ui, targets, rect, label, action, accent, None);
}

/// The version shown at the bottom right. Change it with each release.
const APP_VERSION: &str = "0.1.2";

/// The stone grey of every button that is not chosen.
const GRAY: u32 = 0x5a5a5a;

/// Colours of the section buttons (under the top bar): the same grey, and a light violet when chosen.
const SECTION: (u32, u32) = (GRAY, 0x7b4fe0);

/// The light pink-violet of a switch that is ON.
pub(super) const SWITCH_ON: u32 = 0xa855e8;

/// `button` with its own plate colours (idle, chosen) in place of the top bar's grey and purple.
pub(super) fn button_as(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
    tint: Option<(u32, u32)>,
) {
    let main = matches!(action, Action::Play);
    let (x, y, w, h) = plate_rect(rect, main);
    // Grey stone; the chosen button is purple; Play is the big purple main block, cracked.
    block_plate(
        ui,
        x,
        y,
        w,
        h,
        match tint {
            Some((idle, chosen)) => {
                if accent {
                    chosen
                } else {
                    idle
                }
            }
            None => {
                if main || accent {
                    0x7b4fe0
                } else {
                    GRAY
                }
            }
        },
        main,
    );
    let padding = (w * 0.12).clamp(4.0, 12.0);
    let base = if main {
        34.0
    } else if h >= 60.0 {
        26.0
    } else if w < 80.0 {
        13.0
    } else if h >= 50.0 {
        17.0
    } else if h >= 44.0 && !matches!(action, Action::Account) {
        18.0
    } else {
        16.0
    };
    let font = if matches!(action, Action::Page(_) | Action::ImportWorld) {
        (base * ((w - 2.0 * padding) / ui.measure(Face::Medium, base, label).max(1.0)).min(1.0))
            .max(9.0)
    } else {
        base
    };
    let label = ui.fit_text(Face::Medium, font, label, w - 2.0 * padding);
    // The label sits in the middle of the plate, across and up and down.
    let text_x = x + ((w - ui.measure(Face::Medium, font, &label)) / 2.0).max(2.0);
    shadow_text(
        ui,
        Face::Medium,
        font,
        text_x,
        y + ((h - font * 1.45) / 2.0).max(2.0),
        &label,
    );
    targets.push((rect, action));
}
/// A mod's ON (green) or OFF (red) block switch: one press flips it.
fn switch_button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    on: bool,
    action: Action,
) {
    // The word, thin and white, inside a clear glass pill whose right edge is the rows' edge.
    // Narrower than its row's slot and moved left, away from the side-menu arrow.
    let rect = [rect[0] + rect[2] - 24.0 - 96.0, rect[1], 96.0, rect[3]];
    let (px, py, pw, ph) = plate_rect(rect, false);
    block_plate(ui, px, py, pw, ph, GRAY, false);
    let [x, y, w, h] = rect;
    let label = if on { "ON" } else { "OFF" };
    let size = 22.0;
    let width = ui.measure(Face::Regular, size, label);
    ui.text(
        Face::Regular,
        size,
        x + (w - width) / 2.0,
        y + ((h - size * 1.45) / 2.0).max(6.0),
        color::hex(0xffffff, 255),
        label,
    );
    targets.push((rect, action));
}
fn toggle(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    on: bool,
    action: Action,
) {
    switch_button(ui, targets, rect, on, action);
}
impl Launcher {
    /// The same spatial focus algorithm as the game's controller menus.
    pub(super) fn navigate(&mut self, key: &KeyEvent) -> bool {
        self.navigate_key(&key.logical_key)
    }
    pub(super) fn navigate_key(&mut self, key: &winit::keyboard::Key) -> bool {
        use horizon_client::gamepad::{self, Button};
        use winit::keyboard::{Key, NamedKey};
        if self.pending_focus.is_some() {
            return true;
        }
        if self.catalog.editing.is_some() || self.targets.is_empty() {
            return false;
        }
        let button = match key {
            Key::Named(NamedKey::ArrowUp) => Button::DPadUp,
            Key::Named(NamedKey::ArrowDown) => Button::DPadDown,
            Key::Named(NamedKey::ArrowLeft) => Button::DPadLeft,
            Key::Named(NamedKey::ArrowRight) => Button::DPadRight,
            Key::Named(NamedKey::Tab) => {
                self.focus = Some(self.focus.map_or(0, |i| (i + 1) % self.targets.len()));
                return true;
            }
            _ => return false,
        };
        if matches!(button, Button::DPadUp | Button::DPadDown)
            && let Some((_, action)) = self.focus.and_then(|i| self.targets.get(i))
            && let Some(index) = self.list_targets.iter().position(|(_, a)| a == action)
        {
            let rects: Vec<_> = self.list_targets.iter().map(|(r, _)| *r).collect();
            if let Some(next) = gamepad::focus_step_strict(rects[index], button, &rects)
                && let Some(view) = self.list_view
            {
                let (rect, action) = self.list_targets[next];
                let scroll = if let Some(chooser) = &mut self.chooser {
                    &mut chooser.scroll
                } else {
                    &mut self.catalog.scroll
                };
                *scroll = super::gesture::reveal(*scroll, rect, view);
                self.pending_focus = Some(action);
                self.focus = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return true;
            }
        }
        self.focus = Some(
            if let Some(index) = self.focus.filter(|i| *i < self.targets.len()) {
                let rects: Vec<_> = self.targets.iter().map(|(rect, _)| *rect).collect();
                gamepad::focus_step_rect(rects[index], button, &rects).unwrap_or(index)
            } else {
                0
            },
        );
        true
    }

    pub(super) fn draw_pages(&mut self) -> Result<()> {
        let (Some(r), Some(gui)) = (&mut self.renderer, &self.gui) else {
            return Ok(());
        };
        if r.suspended {
            return Ok(());
        }
        // The launcher is set in block lettering; the game keeps its own fonts.
        horizon_client::ui_theme::set_block_letters(!GLASS);
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        // OEM navigation bars can remain visible even in fullscreen (keypad phones).
        // Inset the launcher layout only; keep surface and input coordinates intact.
        if let Some(window) = &self.window {
            use winit::platform::android::WindowExtAndroid;
            let rect = window.content_rect();
            let input = winit::platform::android::native_input_scale();
            if rect.bottom > 0 {
                ui.h = ui.h.min(rect.bottom as f32 * input[1] as f32 / ui.density);
            }
            if rect.right > 0 {
                ui.w = ui.w.min(rect.right as f32 * input[0] as f32 / ui.density);
            }
        }
        // The sign-in pages keep their size under the keyboard: nothing moves when a field is tapped.
        if !self.login.visible
            && let Some(bottom) = crate::native_ime::viewport_bottom()
        {
            ui.h = ui.h.min(bottom as f32 / ui.density);
        }
        let (w, h) = (ui.w, ui.h);
        // The background: a plain, deep dark grey, no shapes.
        ui.rounded(0.0, 0.0, w, h, 0.0, color::hex(0x13161b, 255));
        // An Enderman's eyes in the middle, drawn as a few rectangles (the picture is
        // 2576 x 1449, cropped to fill the screen); the home screen has its own plain look.
        let home = self.catalog.page == Page::Play
            && horizon_client::official_install::ready(&self.home)
            && self.worker.is_none();
        let sunset = !home;
        if sunset {
            super::sunset::draw(&mut ui, w, h);
        }
        if !home && !sunset {
            let scale = ui.density / ui.batch.scale;
            ui.batch.fill(
                &gui.atlas,
                0.0,
                0.0,
                w * scale,
                h * scale,
                color::hex(0x13161b, 255),
            );
            let fit = (w / 2576.0).max(h / 1449.0);
            let at = |x: f32, y: f32| {
                (
                    (w / 2.0 + (x - 1288.0) * fit) * scale,
                    (h / 2.0 + (y - 724.0) * fit) * scale,
                )
            };
            // Per eye: where its dim, bright and dim parts start, and where it ends.
            for edges in [
                [746.0, 886.0, 1073.0, 1186.0],
                [1391.0, 1503.0, 1689.0, 1830.0],
            ] {
                for (part, shade) in [(0, 0x9a449b), (1, 0xd700fc), (2, 0x9a449b)] {
                    let (x0, y0) = at(edges[part], 646.0);
                    let (x1, y1) = at(edges[part + 1], 803.0);
                    ui.batch
                        .fill(&gui.atlas, x0, y0, x1 - x0, y1 - y0, color::hex(shade, 235));
                }
            }
        }
        if home {
            if self.wardrobe.needs_open() {
                self.wardrobe.open(
                    &self.catalog.config.name,
                    &self.catalog.config.microsoft_client_id,
                );
            }
            // A slow purple hologram: a gradient, drifting light waves and twinkling dust.
            static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            let t = START
                .get_or_init(std::time::Instant::now)
                .elapsed()
                .as_secs_f32()
                * 0.6;
            let bands = 80;
            for i in 0..bands {
                let f = i as f32 / (bands - 1) as f32;
                let rgb = super::sunset::sky_at(f as f64 * 0.92);
                let band = h / bands as f32;
                ui.rounded(
                    0.0,
                    i as f32 * band,
                    w,
                    band + 1.0,
                    0.0,
                    color::hex(rgb, 255),
                );
            }
            for k in 0..5 {
                let kf = k as f32;
                let base = h * (0.36 + 0.1 * kf);
                let amp = 22.0 + 6.0 * kf;
                let mut x = 0.0;
                while x < w {
                    let edge = (std::f32::consts::PI * x / w).sin();
                    let y = base
                        + amp * (x * 0.011 + t * (0.5 + 0.06 * kf) + kf * 0.9).sin()
                        + amp * 0.4 * (x * 0.026 - t * 0.35 + kf).sin();
                    let alpha = (edge * (150.0 - kf * 20.0)).max(0.0);
                    // A soft glow under a fine core, so no step shows along the line.
                    ui.rounded(
                        x,
                        y - 4.0,
                        3.4,
                        9.0,
                        2.0,
                        color::hex(super::sunset::WAVE, (alpha * 0.2) as u8),
                    );
                    ui.rounded(
                        x,
                        y,
                        3.4,
                        1.8,
                        0.9,
                        color::hex(super::sunset::WAVE, alpha as u8),
                    );
                    x += 3.0;
                }
            }
            for i in 0..14 {
                let fi = i as f32;
                let x = (fi * 0.618_034 % 1.0) * w;
                let speed = 5.0 + (i % 3) as f32 * 4.0;
                let y = ((fi * 0.381_966 % 1.0) * h - t * speed).rem_euclid(h);
                let fade = (std::f32::consts::PI * y / h).sin();
                let alpha = (0.5 + 0.5 * (t * 1.2 + fi * 1.7).sin()) * 200.0 * fade;
                let size = 3.0 + (i % 2) as f32;
                // A round glow with a small bright middle.
                ui.rounded(
                    x - size * 1.2,
                    y - size * 1.2,
                    size * 3.4,
                    size * 3.4,
                    size * 1.7,
                    color::hex(super::sunset::DUST, (alpha * 0.18) as u8),
                );
                ui.rounded(
                    x,
                    y,
                    size,
                    size,
                    size / 2.0,
                    color::hex(super::sunset::DUST, alpha as u8),
                );
            }
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            let surface = color::hex(0x17161a, 255);
            let card = color::hex(0x2b2930, 255);
            let white = color::hex(0xffffff, 255);
            let gray = color::hex(0x8e8e8e, 255);
            // Top bar: the account on the left, settings on the right.
            ui.rounded(0.0, 0.0, w, 70.0, 0.0, surface);
            let account = crate::account::Accounts::load().selected().cloned();
            let (who, provider) = match &account {
                Some(a) => (
                    a.name.clone(),
                    if a.provider == crate::account::Provider::Ely {
                        "Ely.by"
                    } else {
                        "Microsoft"
                    },
                ),
                None => (self.catalog.config.name.clone(), "Offline"),
            };
            // The account on the left; on the right the name of the app, then the gear.
            let chip_w = (w * 0.4).min(260.0);
            ui.rounded(24.0, 13.0, chip_w, 44.0, 22.0, card);
            self.wardrobe.draw_face(&mut ui, 29.0, 18.0, 34.0);
            let who = ui.fit_text(Face::Medium, 16.0, &who, chip_w - 56.0);
            ui.text(Face::Medium, 16.0, 72.0, 17.0, white, &who);
            ui.text(Face::Regular, 12.0, 72.0, 36.0, gray, provider);
            self.targets
                .push(([24.0, 13.0, chip_w, 44.0], Action::Account));
            ui.rounded(w - 64.0, 13.0, 44.0, 44.0, 22.0, card);
            gear_icon(&mut ui, w - 56.0, 21.0, 28.0);
            self.targets
                .push(([w - 64.0, 13.0, 44.0, 44.0], Action::Page(Page::Settings)));
            let name_w = ui.measure(Face::Medium, 22.0, "Horizon");
            ui.text(
                Face::Medium,
                22.0,
                w - 64.0 - 38.0 - name_w,
                22.0,
                white,
                "Horizon",
            );
            // Bottom bar: what is installed on the left, Play on the right.
            ui.rounded(0.0, h - 80.0, w, 80.0, 0.0, color::hex(0x000000, 205));
            ui.rounded(0.0, h - 80.0, w, 1.0, 0.0, color::hex(0xffffff, 28));
            if !self.catalog.status.is_empty() {
                let status = ui.fit_text(Face::Regular, 13.0, &self.catalog.status, w - 32.0);
                ui.text(Face::Regular, 13.0, 16.0, h - 106.0, gray, &status);
            }
            let version = format!("v{APP_VERSION}");
            let version_w = ui.measure(Face::Medium, 20.0, &version);
            ui.text(
                Face::Medium,
                20.0,
                w - 16.0 - version_w,
                h - 55.0,
                gray,
                &version,
            );
            let play_w = (w * 0.3).min(230.0);
            let play = [14.0, h - 66.0, play_w, 52.0];
            let starting = self.session_worker.is_some();
            ui.rounded(
                play[0],
                play[1],
                play[2],
                play[3],
                20.0,
                if starting {
                    color::hex(0xbdbdbd, 255)
                } else {
                    white
                },
            );
            let label = if starting { "Starting..." } else { "PLAY" };
            let lw = ui.measure(Face::Medium, 20.0, label);
            ui.text(
                Face::Medium,
                20.0,
                play[0] + (play[2] - lw) / 2.0,
                play[1] + 11.0,
                color::hex(0x111111, 255),
                label,
            );
            self.targets.push((play, Action::Play));
        } else {
            {
                // The same top bar as the home screen: account, the name, settings.
                if self.wardrobe.needs_open() {
                    self.wardrobe.open(
                        &self.catalog.config.name,
                        &self.catalog.config.microsoft_client_id,
                    );
                }
                let card = color::hex(0x2b2930, 255);
                let white = color::hex(0xffffff, 255);
                let gray = color::hex(0x8e8e8e, 255);
                ui.rounded(0.0, 0.0, w, 70.0, 0.0, color::hex(0x17161a, 255));
                let account = crate::account::Accounts::load().selected().cloned();
                let (who, provider) = match &account {
                    Some(a) => (
                        a.name.clone(),
                        if a.provider == crate::account::Provider::Ely {
                            "Ely.by"
                        } else {
                            "Microsoft"
                        },
                    ),
                    None => (self.catalog.config.name.clone(), "Offline"),
                };
                let chip_w = (w * 0.4).min(260.0);
                ui.rounded(24.0, 13.0, chip_w, 44.0, 22.0, card);
                self.wardrobe.draw_face(&mut ui, 29.0, 18.0, 34.0);
                let who = ui.fit_text(Face::Medium, 16.0, &who, chip_w - 56.0);
                ui.text(Face::Medium, 16.0, 72.0, 17.0, white, &who);
                ui.text(Face::Regular, 12.0, 72.0, 36.0, gray, provider);
                self.targets
                    .push(([24.0, 13.0, chip_w, 44.0], Action::Account));
                ui.rounded(w - 64.0, 13.0, 44.0, 44.0, 22.0, card);
                gear_icon(&mut ui, w - 56.0, 21.0, 28.0);
                self.targets
                    .push(([w - 64.0, 13.0, 44.0, 44.0], Action::Page(Page::Settings)));
                let name_w = ui.measure(Face::Medium, 22.0, "Horizon");
                ui.text(
                    Face::Medium,
                    22.0,
                    w - 64.0 - 38.0 - name_w,
                    22.0,
                    white,
                    "Horizon",
                );
                self.targets.push((
                    [w - 64.0 - 38.0 - name_w - 8.0, 14.0, name_w + 16.0, 42.0],
                    Action::Page(Page::Play),
                ));
                if matches!(
                    self.catalog.page,
                    Page::Mods | Page::Shaders | Page::Packs | Page::Skins
                ) || (self.catalog.page == Page::Settings && w >= 700.0)
                {
                    // The bottom bar: Eject (add a file) where the home screen has Play.
                    ui.rounded(0.0, h - 80.0, w, 80.0, 0.0, color::hex(0x000000, 205));
                    ui.rounded(0.0, h - 80.0, w, 1.0, 0.0, color::hex(0xffffff, 28));
                    let version = format!("v{APP_VERSION}");
                    let version_w = ui.measure(Face::Medium, 20.0, &version);
                    ui.text(
                        Face::Medium,
                        20.0,
                        w - 16.0 - version_w,
                        h - 55.0,
                        gray,
                        &version,
                    );
                    let eject = [14.0, h - 66.0, (w * 0.3).min(230.0), 52.0];
                    ui.rounded(eject[0], eject[1], eject[2], eject[3], 20.0, white);
                    // Settings has "Back" in the white button; the others add a file.
                    let word = if self.catalog.page == Page::Settings {
                        "Back"
                    } else {
                        "Eject"
                    };
                    let ew = ui.measure(Face::Medium, 20.0, word);
                    ui.text(
                        Face::Medium,
                        20.0,
                        eject[0] + (eject[2] - ew) / 2.0,
                        eject[1] + 11.0,
                        color::hex(0x111111, 255),
                        word,
                    );
                    let kind = match self.catalog.page {
                        Page::Mods => horizon_client::content_files::Kind::Mods,
                        Page::Shaders => horizon_client::content_files::Kind::Shaders,
                        _ => horizon_client::content_files::Kind::Packs,
                    };
                    self.targets.push((
                        eject,
                        if self.catalog.page == Page::Settings {
                            Action::Page(Page::Play)
                        } else if self.catalog.page == Page::Skins {
                            Action::WardrobeImport
                        } else {
                            Action::Import(kind)
                        },
                    ));
                }
            }
            // The install screen has the new top bar and no old section row.
            let chips_page = self.catalog.page == Page::Play
                || w >= 700.0
                    && (self.catalog.page.is_catalog()
                        || self.catalog.page == Page::Installed
                        || self.catalog.page == Page::Skins
                        || self.catalog.page == Page::Settings);
            let tabs = Page::ALL.len() as f32;
            let tab_margin = if w < 700.0 { 16.0 } else { 32.0 };
            let tab_width = ((w - 2.0 * tab_margin - 16.0) / tabs).min(150.0);
            for (i, page) in Page::ALL.iter().enumerate().filter(|_| !chips_page) {
                let x = tab_margin + i as f32 * (tab_width + 8.0);
                button(
                    &mut ui,
                    &mut self.targets,
                    if w < 700.0 {
                        [x, 84.0, tab_width, 46.0]
                    } else {
                        [x, 74.0, tab_width, 60.0]
                    },
                    page.name(),
                    Action::Page(*page),
                    if *page == Page::Mods {
                        self.catalog.page.is_add_on()
                    } else {
                        *page == self.catalog.page
                    },
                );
            }
        }
        let compact = w < 700.0;
        let page = self.catalog.page;
        if home {
        } else if page == Page::Play {
            use horizon_client::official_install::InstallStage;
            // The version, bottom right, as on the other screens.
            let version = format!("v{APP_VERSION}");
            let version_w = ui.measure(Face::Medium, 20.0, &version);
            ui.text(
                Face::Medium,
                20.0,
                w - 16.0 - version_w,
                h - 38.0,
                color::hex(0x8e8e8e, 255),
                &version,
            );
            let installed = horizon_client::official_install::ready(&self.home);
            let installing = self.worker.is_some();
            let progress = *self.install.lock().unwrap();
            let top = 160.0;
            if installing || !installed {
                ui.text(
                    Face::Medium,
                    32.0,
                    32.0,
                    top,
                    color::TEXT,
                    if installing {
                        "Getting your game ready"
                    } else if compact {
                        "Your Java worlds"
                    } else {
                        "Your Java worlds, on Android"
                    },
                );
            }
            if installing {
                let current = InstallStage::ALL
                    .iter()
                    .position(|s| *s == progress.stage)
                    .unwrap_or(0);
                let row_height = ((h - 360.0) / 6.0).clamp(25.0, 40.0);
                if h >= 620.0 {
                    for (index, stage) in InstallStage::ALL.iter().enumerate() {
                        let y = top + 90.0 + index as f32 * row_height;
                        let label = if index < current {
                            format!("{:02}  {}  - done", index + 1, stage.label())
                        } else {
                            format!("{:02}  {}", index + 1, stage.label())
                        };
                        ui.text(
                            Face::Regular,
                            16.0,
                            32.0,
                            y,
                            if index == current {
                                color::TEXT
                            } else {
                                color::MUTED
                            },
                            &label,
                        );
                    }
                } else {
                    ui.text(
                        Face::Regular,
                        18.0,
                        32.0,
                        top + 100.0,
                        color::TEXT,
                        progress.stage.label(),
                    );
                    ui.text(
                        Face::Regular,
                        15.0,
                        32.0,
                        top + 138.0,
                        color::MUTED,
                        &format!("Step {} of 6", current + 1),
                    );
                }
                let fraction = if progress.total > 0 {
                    progress.done as f32 / progress.total as f32
                } else {
                    0.0
                };
                let overall = (current as f32 + fraction.clamp(0.0, 1.0)) / 6.0;
                let bar_y = h - 152.0;
                ui.rounded(32.0, bar_y, w - 64.0, 4.0, 2.0, color::RAISED);
                if overall > 0.0 {
                    ui.rounded(32.0, bar_y, (w - 64.0) * overall, 4.0, 2.0, color::ACCENT);
                }
                let detail = match progress.stage {
                    InstallStage::Download => format!(
                        "{:.1} / {:.1} MiB  - verified downloads",
                        progress.done as f64 / 1048576.0,
                        progress.total as f64 / 1048576.0
                    ),
                    InstallStage::Controls => format!(
                        "{} / {} control files verified",
                        progress.done, progress.total
                    ),
                    _ => "Please keep Horizon open. Downloaded files are kept for retry.".into(),
                };
                ui.text(
                    Face::Regular,
                    14.0,
                    32.0,
                    bar_y + 12.0,
                    color::MUTED,
                    &detail,
                );
            }
            if installing {
                ui.text(
                    Face::Medium,
                    19.0,
                    32.0,
                    h - 90.0,
                    color::TEXT,
                    progress.stage.label(),
                );
            } else {
                button(
                    &mut ui,
                    &mut self.targets,
                    [32.0, h - 118.0, (w - 64.0).min(400.0), 100.0],
                    if self.session_worker.is_some() {
                        "Starting..."
                    } else if installed {
                        "Play"
                    } else if self.error.is_some() {
                        "Retry installation"
                    } else {
                        "Download Minecraft 26.3"
                    },
                    Action::Play,
                    true,
                );
            }
        } else if page == Page::Skins {
            if !compact {
                section_chips(&mut ui, &mut self.targets, w, h, 0.0, page);
            }
            self.wardrobe
                .draw(&mut ui, &mut self.targets, self.catalog.scroll);
            self.list_view = Some(self.wardrobe.view);
            self.catalog.scroll = self.catalog.scroll.min(self.wardrobe.view[2]);
        } else if page.is_catalog() {
            section_chips(&mut ui, &mut self.targets, w, h, 0.0, page);
            // The search field: a plain outlined box (white rim, the dark page inside).
            {
                let rect = if compact {
                    [32.0, 174.0, w - 214.0, 44.0]
                } else {
                    [32.0, 126.0, w - 216.0, 40.0]
                };
                // Inner dark field and a rim of the same dark grey on all four sides.
                let rim = color::hex(GRAY, 255);
                ui.rounded(
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    0.0,
                    color::hex(0x0f1013, 255),
                );
                ui.border(rect[0], rect[1], rect[2], rect[3], 0.0, rim);
                let shown = if self.catalog.query.is_empty() {
                    if page == Page::Mods {
                        "Search mod name".to_string()
                    } else {
                        "Search".to_string()
                    }
                } else {
                    self.catalog.query.clone()
                };
                let shown = ui.fit_text(Face::Medium, 18.0, &shown, rect[2] - 28.0);
                ui.text(
                    Face::Medium,
                    18.0,
                    rect[0] + 14.0,
                    rect[1] + 11.0,
                    color::hex(0xffffff, 255),
                    &shown,
                );
                self.targets.push((rect, Action::Field("search")));
            }
            button_as(
                &mut ui,
                &mut self.targets,
                if compact {
                    [w - 176.0, 174.0, 120.0, 44.0]
                } else {
                    [w - 176.0, 126.0, 120.0, 40.0]
                },
                if page == Page::Mods && self.catalog.query.trim().starts_with("https://") {
                    "Import .hmod"
                } else {
                    "Search"
                },
                Action::Search,
                true,
                Some(SECTION),
            );
            let kind = match page {
                Page::Mods => horizon_client::content_files::Kind::Mods,
                Page::Shaders => horizon_client::content_files::Kind::Shaders,
                _ => horizon_client::content_files::Kind::Packs,
            };
            let file_button_width = ((w - 72.0) / 2.0).min(158.0);
            // On a wide screen the white Eject button at the bottom adds files instead.
            if compact {
                button(
                    &mut ui,
                    &mut self.targets,
                    [32.0, 222.0, file_button_width, 38.0],
                    "Import file",
                    Action::Import(kind),
                    false,
                );
            }
            let bundled: Vec<_> = if page == Page::Mods
                && self.catalog.config.mod_source == crate::shader_catalog::Origin::Horizon
            {
                let query = self.catalog.query.to_lowercase();
                horizon_client::mod_list::ANDROID_BUNDLED
                    .into_iter()
                    .filter(|(id, title, _)| {
                        id.contains(&query) || title.to_lowercase().contains(&query)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let catalog_rows: Vec<_> = self
                .catalog
                .results
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    !(page == Page::Mods
                        && p.source == crate::browser::SourceId::Horizon
                        && horizon_client::mod_list::ANDROID_BUNDLED
                            .iter()
                            .any(|(id, _, _)| *id == p.id))
                        // `.hnmod` source mods cannot be installed here: no row.
                        && !(page == Page::Mods && crate::catalog::source_only_mod(p))
                })
                .collect();
            // Mods added from files or downloads, in the same list as the built-in ones.
            let local_mods: Vec<usize> = if page == Page::Mods {
                self.catalog
                    .local
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.kind == horizon_client::content_files::Kind::Mods)
                    .map(|(i, _)| i)
                    .collect()
            } else {
                Vec::new()
            };
            let row_count = bundled.len() + local_mods.len() + catalog_rows.len();
            let top = if compact { 270.0 } else { 184.0 };
            let pitch = if compact { 86.0 } else { 66.0 };
            let bottom = if compact { h - 112.0 } else { h - 130.0 };
            let tile_h = if compact { 76.0 } else { 56.0 };
            self.catalog.scroll = self
                .catalog
                .scroll
                .clamp(0.0, (row_count as f32 * pitch - (bottom - top)).max(0.0));
            self.list_view = Some([
                top,
                bottom,
                (row_count as f32 * pitch - (bottom - top)).max(0.0),
            ]);
            // A row half outside the list shows only its inside half.
            let clip_from = ui.mark();
            for (row, (id, title, description)) in bundled.iter().enumerate() {
                let y = top + row as f32 * pitch - self.catalog.scroll;
                if y + tile_h <= top || y >= bottom {
                    continue;
                }
                // Zoom has its own picture, the same one the game's mod list shows.
                if *id == "zoom" && !self.catalog.mod_icons.contains_key("bundled:zoom") {
                    let squares = crate::catalog::icon_squares_from(include_bytes!(
                        "../game/android_bundled/zoom/icon.png"
                    ));
                    self.catalog
                        .mod_icons
                        .insert("bundled:zoom".into(), squares);
                }
                let picture = self
                    .catalog
                    .mod_icons
                    .get(&format!("bundled:{id}"))
                    .filter(|squares| !squares.is_empty());
                if let Some(squares) = picture {
                    ui.rounded(32.0, y + 2.0, 60.0, 60.0, 0.0, color::hex(0xffffff, 255));
                    ui.rounded(33.0, y + 3.0, 58.0, 58.0, 0.0, color::hex(0x0f1013, 255));
                    for ([sx, sy, sw, sh], c) in squares {
                        ui.rounded(
                            33.0 + sx * 1.8125,
                            y + 3.0 + sy * 1.8125,
                            sw * 1.8125 + 0.3,
                            sh * 1.8125 + 0.3,
                            0.0,
                            *c,
                        );
                    }
                } else {
                    mod_tile(&mut ui, 32.0, y, title);
                }
                let shown = ui.fit_text(Face::Medium, 22.0, title, w - 286.0);
                let summary = ui.fit_text(Face::Regular, 16.0, description, w - 286.0);
                ui.text(Face::Medium, 22.0, 104.0, y, color::TEXT, &shown);
                ui.text(Face::Regular, 14.0, 104.0, y + 33.0, color::MUTED, &summary);
                switch_button(
                    &mut ui,
                    &mut self.targets,
                    [w - 150.0, y, 118.0, 46.0],
                    self.catalog.bundled_enabled.contains(*id),
                    Action::ToggleBundled(id),
                );
                let dots = if w < 700.0 {
                    [w - 150.0, y + 50.0, 56.0, 30.0]
                } else {
                    [w - 216.0, y, 56.0, 46.0]
                };
                more_dots(&mut ui, &mut self.targets, dots, Action::BuiltInInfo);
            }
            for (row, i) in local_mods.iter().copied().enumerate() {
                let y = top + (row + bundled.len()) as f32 * pitch - self.catalog.scroll;
                // The three dots open the "Remove add-on?" question; on a narrow screen they sit under the switch.
                let remove = if w < 700.0 {
                    [w - 150.0, y + 50.0, 56.0, 30.0]
                } else {
                    [w - 216.0, y, 56.0, 46.0]
                };
                let title_room = if w < 700.0 { w - 214.0 } else { w - 240.0 };
                self.list_targets
                    .push(([w - 150.0, y, 118.0, 46.0], Action::ToggleLocal(i)));
                self.list_targets.push((remove, Action::DeleteLocal(i)));
                if y + tile_h <= top || y >= bottom {
                    continue;
                }
                let item = &self.catalog.local[i];
                let picture = self
                    .catalog
                    .mod_icons
                    .get(&item.file)
                    .filter(|squares| !squares.is_empty());
                if let Some(squares) = picture {
                    // The picture of the mod from its Modrinth page, in the same frame as the others.
                    ui.rounded(32.0, y + 2.0, 60.0, 60.0, 0.0, color::hex(0xffffff, 255));
                    ui.rounded(33.0, y + 3.0, 58.0, 58.0, 0.0, color::hex(0x0f1013, 255));
                    for ([sx, sy, sw, sh], c) in squares {
                        ui.rounded(
                            33.0 + sx * 1.8125,
                            y + 3.0 + sy * 1.8125,
                            sw * 1.8125 + 0.3,
                            sh * 1.8125 + 0.3,
                            0.0,
                            *c,
                        );
                    }
                } else {
                    mod_tile(&mut ui, 32.0, y, &item.shown);
                }
                let title = ui.fit_text(Face::Medium, 22.0, &item.shown, title_room - 72.0);
                ui.text(Face::Medium, 22.0, 104.0, y, color::TEXT, &title);
                ui.text(
                    Face::Regular,
                    14.0,
                    104.0,
                    y + 33.0,
                    color::MUTED,
                    "Added mod",
                );
                if cfg!(target_pointer_width = "64") {
                    switch_button(
                        &mut ui,
                        &mut self.targets,
                        [w - 150.0, y, 118.0, 46.0],
                        item.enabled,
                        Action::ToggleLocal(i),
                    );
                } else {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        "64-bit required",
                    );
                }
                more_dots(&mut ui, &mut self.targets, remove, Action::DeleteLocal(i));
            }
            for (row, (i, p)) in catalog_rows.into_iter().enumerate() {
                let y = top + (row + bundled.len() + local_mods.len()) as f32 * pitch
                    - self.catalog.scroll;
                let download = self.catalog.download_label(p);
                if download.is_none()
                    && !crate::catalog::source_only_mod(p)
                    && !(p.kind == crate::browser::Kind::Mod
                        && p.source == crate::browser::SourceId::Modrinth)
                {
                    self.list_targets
                        .push(([w - 150.0, y, 118.0, 78.0], Action::Install(i)));
                }
                if y + 78.0 <= top || y >= bottom {
                    continue;
                }
                // The project's picture, in a small framed square (a plain one until it arrives).
                ui.rounded(32.0, y + 2.0, 60.0, 60.0, 0.0, color::hex(0xb9bac2, 255));
                ui.rounded(33.0, y + 3.0, 58.0, 58.0, 0.0, color::hex(0x0f1013, 255));
                if let Some(squares) = p.icon.as_ref().and_then(|u| self.catalog.icons.get(u)) {
                    for ([sx, sy, sw, sh], c) in squares {
                        ui.rounded(
                            33.0 + sx * 1.8125,
                            y + 3.0 + sy * 1.8125,
                            sw * 1.8125 + 0.3,
                            sh * 1.8125 + 0.3,
                            0.0,
                            *c,
                        );
                    }
                }
                let title = ui.fit_text(Face::Medium, 22.0, &p.title, w - 282.0);
                let summary = ui.fit_text(Face::Regular, 16.0, &p.summary, w - 282.0);
                ui.text(Face::Medium, 22.0, 104.0, y, color::TEXT, &title);
                ui.text(Face::Regular, 14.0, 104.0, y + 33.0, color::MUTED, &summary);
                if let Some(state) = download {
                    ui.text(Face::Medium, 16.0, w - 150.0, y + 12.0, color::TEXT, &state);
                } else if p.kind == crate::browser::Kind::Mod
                    && p.source == crate::browser::SourceId::Modrinth
                {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        "Java mod",
                    );
                } else if crate::catalog::source_only_mod(p) {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        ".hnmod: later",
                    );
                } else if p.kind == crate::browser::Kind::Mod && !cfg!(target_pointer_width = "64")
                {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        "64-bit required",
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 150.0, y, 118.0, 46.0],
                        self.catalog.install_button(p),
                        Action::Install(i),
                        false,
                    );
                }
            }
            ui.clip_since(clip_from, 0.0, top, w, bottom);
            button(
                &mut ui,
                &mut self.targets,
                [
                    32.0,
                    if compact { h - 100.0 } else { h - 124.0 },
                    120.0,
                    40.0,
                ],
                "Previous",
                Action::Previous,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [
                    166.0,
                    if compact { h - 100.0 } else { h - 124.0 },
                    120.0,
                    40.0,
                ],
                "Next",
                Action::Next,
                false,
            );
        } else if page == Page::Installed {
            use horizon_client::content_files::Kind;
            section_chips(&mut ui, &mut self.targets, w, h, 0.0, page);
            let top = if compact { 176.0 } else { 130.0 };
            let bottom = h - 100.0;
            let row_height = if compact { 124.0 } else { 80.0 };
            let total = Kind::ALL
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    54.0 + if self.catalog.sections[i] {
                        self.catalog.local.iter().filter(|e| e.kind == *k).count() as f32
                            * row_height
                    } else {
                        0.0
                    }
                })
                .sum::<f32>();
            let maximum = (total - (bottom - top)).max(0.0);
            self.catalog.scroll = self.catalog.scroll.clamp(0.0, maximum);
            self.list_view = Some([top, bottom, maximum]);
            let mut offset = 0.0;
            for (section, kind) in Kind::ALL.into_iter().enumerate() {
                let count = self.catalog.local.iter().filter(|e| e.kind == kind).count();
                let y = top + offset - self.catalog.scroll;
                let label = format!(
                    "{} {} ({count})",
                    if self.catalog.sections[section] {
                        "-"
                    } else {
                        "+"
                    },
                    kind.label()
                );
                self.list_targets
                    .push(([32.0, y, w - 210.0, 46.0], Action::ToggleSection(section)));
                self.list_targets
                    .push(([w - 165.0, y, 133.0, 46.0], Action::Import(kind)));
                if y >= top && y + 46.0 <= bottom {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [32.0, y, w - 210.0, 46.0],
                        &label,
                        Action::ToggleSection(section),
                        false,
                    );
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 165.0, y, 133.0, 46.0],
                        "Import",
                        Action::Import(kind),
                        false,
                    );
                }
                offset += 54.0;
                if !self.catalog.sections[section] {
                    continue;
                }
                for (i, item) in self
                    .catalog
                    .local
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.kind == kind)
                {
                    let y = top + offset - self.catalog.scroll;
                    offset += row_height;
                    self.list_targets.push((
                        [
                            if compact { 32.0 } else { w - 300.0 },
                            y,
                            120.0,
                            row_height - 4.0,
                        ],
                        Action::ToggleLocal(i),
                    ));
                    self.list_targets.push((
                        [w - 165.0, y, 133.0, row_height - 4.0],
                        Action::DeleteLocal(i),
                    ));
                    if y < top || y + row_height - 4.0 > bottom {
                        continue;
                    }
                    let title = ui.fit_text(
                        Face::Medium,
                        22.0,
                        &item.title,
                        if compact { w - 64.0 } else { w - 364.0 },
                    );
                    ui.text(Face::Medium, 22.0, 32.0, y, color::TEXT, &title);
                    ui.text(
                        Face::Regular,
                        16.0,
                        32.0,
                        y + 32.0,
                        color::MUTED,
                        kind.label(),
                    );
                    if kind == Kind::Mods && !cfg!(target_pointer_width = "64") {
                        ui.text(
                            Face::Regular,
                            14.0,
                            if compact { 32.0 } else { w - 300.0 },
                            y + if compact { 70.0 } else { 12.0 },
                            color::MUTED,
                            "64-bit required",
                        );
                    } else {
                        toggle(
                            &mut ui,
                            &mut self.targets,
                            [
                                if compact { 32.0 } else { w - 300.0 },
                                y + if compact { 62.0 } else { 0.0 },
                                120.0,
                                46.0,
                            ],
                            item.enabled,
                            Action::ToggleLocal(i),
                        );
                    }
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 165.0, y + if compact { 62.0 } else { 0.0 }, 133.0, 46.0],
                        "Remove",
                        Action::DeleteLocal(i),
                        false,
                    );
                }
            }
        } else {
            // On a wide screen Settings covers the whole screen: the grey panel edge to edge,
            // and only a Back button in the top left corner.
            let off = if compact { 0.0 } else { -111.0 };
            let bot = if compact { 130.0 } else { 80.0 };
            if !compact {
                self.targets.clear();
                super::sunset::draw(&mut ui, w, h);
                ui.rounded(0.0, 0.0, w, h, 0.0, color::hex(0x0f1013, 200));
                button(
                    &mut ui,
                    &mut self.targets,
                    [24.0, 16.0, 110.0, 44.0],
                    "Back",
                    Action::Page(Page::Play),
                    false,
                );
            }
            let c = &self.catalog.config;
            ui.text(
                Face::Medium,
                24.0,
                if compact { 32.0 } else { 156.0 },
                if compact { 156.0 } else { 22.0 },
                color::TEXT,
                "Settings",
            );
            if compact {
                ui.rounded(
                    16.0,
                    186.0,
                    w - 32.0,
                    (h - bot) - 186.0,
                    18.0,
                    color::hex(0x0f1013, 200),
                );
            }
            let row_height = if compact { 82.0 } else { 66.0 };
            self.catalog.scroll = self.catalog.scroll.clamp(
                0.0,
                (19.0 * row_height - ((h - bot) - (195.0 + off))).max(0.0),
            );
            self.list_view = Some([
                195.0 + off,
                (h - bot),
                (18.0 * row_height - ((h - bot) - (195.0 + off))).max(0.0),
            ]);
            for (i, (title, value, action)) in [
                ("Rendering", c.renderer.label().to_owned(), Action::Renderer),
                (
                    "Client resolution",
                    if self.catalog.editing == Some("render_scale") {
                        self.catalog.render_scale_text.clone()
                    } else {
                        format!("{}%", c.render_scale.percent())
                    },
                    Action::RenderScale,
                ),
                (
                    "Chunk worker limit",
                    if c.worker_limit == 0 {
                        "No additional limit".into()
                    } else {
                        c.worker_limit.to_string()
                    },
                    Action::WorkerLimit,
                ),
                (
                    "Reusable RAM cache",
                    if c.cache_budget_mib == 0 {
                        "Engine default (80 MiB)".into()
                    } else {
                        format!("{} MiB", c.cache_budget_mib)
                    },
                    Action::CacheBudget,
                ),
                (
                    "Game files",
                    "Open in system Files".into(),
                    Action::GameFiles,
                ),
                (
                    "Worlds: import",
                    "Choose a world ZIP / unpacked here".into(),
                    Action::ImportWorld,
                ),
                (
                    "Worlds: folder",
                    "Open in Files / back up".into(),
                    Action::WorldFiles,
                ),
                ("Profile", format!("Profile {}", c.profile), Action::Profile),
                (
                    "Orientation",
                    c.orientation.label().to_owned(),
                    Action::Orientation,
                ),
                ("Input", c.input_host.label().to_owned(), Action::InputHost),
                (
                    "Immersive fullscreen",
                    if c.screen.immersive { "On" } else { "Off" }.into(),
                    Action::Immersive,
                ),
                (
                    "Avoid camera cutout",
                    if c.screen.avoid_cutout { "On" } else { "Off" }.into(),
                    Action::Cutout,
                ),
                (
                    "Aspect ratio",
                    c.screen.aspect.label().into(),
                    Action::Aspect,
                ),
                (
                    "Display refresh rate",
                    c.device.display_label(),
                    Action::DisplayHz,
                ),
                (
                    "Performance hints",
                    if c.device.performance_hints {
                        "On (ADPF)"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::PerformanceHints,
                ),
                (
                    "GPU profiling",
                    if c.device.gpu_profile {
                        "On / included in report"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::GpuProfile,
                ),
                (
                    "Performance recording",
                    if c.record_performance {
                        "On / up to 30 min"
                    } else {
                        "Off"
                    }
                    .into(),
                    Action::RecordPerformance,
                ),
                (
                    "Diagnostic report",
                    "Export one ZIP to share".into(),
                    Action::ExportDiagnostics,
                ),
                (
                    "Raw performance data",
                    "Advanced / JSONL in Reports".into(),
                    Action::ExportPerformance,
                ),
                (
                    "Server",
                    if c.server.is_empty() {
                        "Singleplayer / tap to enter server".into()
                    } else {
                        c.server.clone()
                    },
                    Action::Field("server"),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let y = 195.0 + off + i as f32 * row_height - self.catalog.scroll;
                self.list_targets.push((
                    [
                        if compact { 32.0 } else { w * 0.36 },
                        y,
                        if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                        if compact { 70.0 } else { 46.0 },
                    ],
                    action,
                ));
                if y < 185.0 + off || y + if compact { 70.0 } else { 46.0 } > (h - bot) {
                    continue;
                }
                ui.text(
                    Face::Regular,
                    16.0,
                    32.0,
                    y + if compact { 0.0 } else { 13.0 },
                    color::MUTED,
                    title,
                );
                if matches!(
                    action,
                    Action::PerformanceHints | Action::Immersive | Action::Cutout
                ) {
                    toggle(
                        &mut ui,
                        &mut self.targets,
                        [
                            if compact { 32.0 } else { w * 0.36 },
                            y + if compact { 24.0 } else { 0.0 },
                            if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                            46.0,
                        ],
                        match action {
                            Action::PerformanceHints => c.device.performance_hints,
                            Action::Immersive => c.screen.immersive,
                            Action::Cutout => c.screen.avoid_cutout,
                            _ => c.device.gpu_profile,
                        },
                        action,
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [
                            if compact { 32.0 } else { w * 0.36 },
                            y + if compact { 24.0 } else { 0.0 },
                            if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                            46.0,
                        ],
                        &format!(
                            "{} {}",
                            value,
                            if matches!(
                                action,
                                Action::Renderer
                                    | Action::Aspect
                                    | Action::DisplayHz
                                    | Action::RenderScale
                                    | Action::Profile
                                    | Action::Orientation
                                    | Action::InputHost
                            ) {
                                "v"
                            } else {
                                ""
                            }
                        ),
                        action,
                        false,
                    );
                }
                ui.rounded(
                    32.0,
                    y + if compact { 78.0 } else { 62.0 },
                    w - 64.0,
                    1.0,
                    0.0,
                    color::BORDER,
                );
            }
            button(
                &mut ui,
                &mut self.targets,
                [
                    32.0,
                    if compact { h - 114.0 } else { h - 62.0 },
                    w - 64.0,
                    42.0,
                ],
                "Reset settings",
                Action::ResetSettings,
                false,
            );
        }
        let status = self.error.as_deref().unwrap_or(&self.catalog.status);
        let status = if (page == Page::Play && self.error.is_none() && self.worker.is_none())
            || (page == Page::Skins && w >= 700.0)
            || (page == Page::Settings && w >= 700.0)
        {
            ""
        } else {
            status
        };
        for (i, line) in status
            .chars()
            .collect::<Vec<_>>()
            .chunks(110)
            .take(2)
            .enumerate()
        {
            // Beside the white button of the bottom bar, not under it.
            let side = if w >= 700.0 {
                (w * 0.3).min(230.0) - 2.0
            } else {
                0.0
            };
            ui.text(
                Face::Regular,
                14.0,
                32.0 + side,
                (if w >= 700.0 { h - 58.0 } else { h - 47.0 }) + i as f32 * 18.0,
                color::MUTED,
                &ui.fit_text(
                    Face::Regular,
                    14.0,
                    &line.iter().collect::<String>(),
                    (w - 210.0 - side).max(40.0),
                ),
            );
        }
        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            if let Some((view, scroll)) = self.login.draw(
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.editing,
                self.catalog.scroll,
            ) {
                self.catalog.scroll = scroll;
                self.list_view = Some(view);
            }
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(&mut ui, &mut self.targets);
            self.list_targets = chooser.navigation(ui.w);
            self.list_view = Some(chooser.viewport(ui.h));
        }
        if let Some([top, bottom, maximum]) = self.list_view
            && maximum > 0.0
            && bottom > top
        {
            let scroll = self
                .chooser
                .as_ref()
                .map_or(self.catalog.scroll, |c| c.scroll);
            let height = bottom - top;
            let thumb = (height * height / (height + maximum)).max(20.0).min(height);
            let y = top + (height - thumb) * (scroll / maximum).clamp(0.0, 1.0);
            ui.rounded(w - 12.0, top, 4.0, height, 2.0, color::BORDER);
            ui.rounded(w - 12.0, y, 4.0, thumb, 2.0, color::MUTED);
        }
        let caret_visible = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .is_ok_and(|t| t.as_millis() % 1000 < 500);
        if caret_visible && let Some(name) = self.catalog.editing {
            let value = match name {
                _ if name.starts_with("ely_") => {
                    let value = self.login.field(name).unwrap_or_default();
                    if name == "ely_username" {
                        value.to_owned()
                    } else {
                        "*".repeat(value.chars().count().min(40))
                    }
                }
                "name" => self.catalog.config.name.clone(),
                "server" => self.catalog.config.server.clone(),
                "render_scale" => self.catalog.render_scale_text.clone(),
                _ => format!("Search: {}", self.catalog.query),
            };
            for ([x, y, w, field_h], action) in &self.targets {
                let y = &(y + (field_h - 42.0) / 2.0);
                if *action == Action::Field(name)
                    || (name == "render_scale" && *action == Action::RenderScale)
                {
                    let text = ui.fit_text(Face::Medium, 16.0, &value, w - 24.0);
                    let caret_x = x + 12.0 + ui.measure(Face::Medium, 16.0, &text);
                    ui.rounded(
                        caret_x.min(x + w - 10.0),
                        y + 12.0,
                        1.0,
                        18.0,
                        0.0,
                        color::TEXT,
                    );
                }
            }
        }
        if self.exit_confirmation {
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            ui.rounded(0.0, 0.0, w, h, 0.0, horizon_client::hud::rgba(0, 0, 0, 160));
            let width = (w - 32.0).min(360.0);
            let x = (w - width) * 0.5;
            let y = (h - 160.0) * 0.5;
            ui.rounded(x, y, width, 160.0, 8.0, color::RAISED);
            ui.text(
                Face::Medium,
                22.0,
                x + 20.0,
                y + 24.0,
                color::TEXT,
                "Exit application?",
            );
            let bw = (width - 52.0) * 0.5;
            button(
                &mut ui,
                &mut self.targets,
                [x + 20.0, y + 92.0, bw, 44.0],
                "Cancel",
                Action::Dismiss,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [x + 32.0 + bw, y + 92.0, bw, 44.0],
                "Exit",
                Action::ConfirmExit,
                false,
            );
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            ui.border(*x - 2.0, *y - 2.0, *w + 4.0, *h + 4.0, 2.0, color::TEXT);
            self.cursor = (
                f64::from((x + w / 2.0) * ui.density),
                f64::from((y + h / 2.0) * ui.density),
            );
        }
        if self.mouse_down || self.drag.as_ref().is_some_and(|drag| !drag.moved) {
            let point = [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ];
            if let Some(([x, y, w, h], _)) = self.targets.iter().rev().find(|(rect, _)| {
                point[0] >= rect[0]
                    && point[0] <= rect[0] + rect[2]
                    && point[1] >= rect[1]
                    && point[1] <= rect[1] + rect[3]
            }) {
                ui.rounded(
                    *x,
                    *y,
                    *w,
                    *h,
                    2.0,
                    horizon_client::hud::rgba(255, 255, 255, 48),
                );
            }
        }
        if !self.mouse_down
            && self.drag.is_none()
            && let Some((action, time)) = self.feedback
            && time.elapsed() < std::time::Duration::from_millis(160)
            && let Some((rect, _)) = self.targets.iter().find(|(_, a)| *a == action)
        {
            let (x, y, w, h) = plate_rect(*rect, matches!(action, Action::Play));
            ui.rounded(x, y, w, h, 0.0, color::hex(0x7b4fe0, 140));
        }
        // A short fade-in when the page changes: a veil of the page colour that lifts.
        if self.shown_page.0 != self.catalog.page {
            self.shown_page = (self.catalog.page, std::time::Instant::now());
        }
        let fade = self.shown_page.1.elapsed().as_secs_f32() / 0.18;
        if fade < 1.0 {
            let alpha = ((1.0 - fade) * (1.0 - fade) * 255.0) as u8;
            ui.rounded(
                0.0,
                134.0,
                w,
                (h - 134.0).max(0.0),
                0.0,
                color::hex(0x13161b, alpha),
            );
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        if home || self.catalog.page.is_add_on() || self.catalog.page == Page::Skins {
            // A tab on the right edge; the sections slide in from the right.
            // While the menu is open only the menu answers taps: not the buttons behind it.
            if self.home_menu {
                self.targets.clear();
                self.list_targets.clear();
            }
            let raw = (self.menu_t.elapsed().as_secs_f32() / 0.26).min(1.0);
            let eased = 1.0 - (1.0 - raw) * (1.0 - raw) * (1.0 - raw);
            let p = if self.home_menu { eased } else { 1.0 - eased };
            if raw < 1.0 {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            let items: [(&str, Action); 5] = [
                ("World", Action::ImportWorld),
                ("Mods", Action::Page(Page::Mods)),
                ("Packs", Action::Page(Page::Packs)),
                ("Skins", Action::Page(Page::Skins)),
                ("Files", Action::Page(Page::Installed)),
            ];
            let (pw, row) = (200.0_f32.min(w * 0.6), 54.0);
            // The panel keeps its size (six rows); the five rows sit together at the top.
            let ph = 6.0 * row + 12.0;
            let py = ((h - ph) / 2.0).max(64.0);
            let px = w - pw * p;
            if p > 0.0 {
                ui.rounded(0.0, 0.0, w, h, 0.0, color::hex(0x000000, (130.0 * p) as u8));
                ui.rounded(px, py, pw + 20.0, ph, 22.0, color::hex(0x000000, 205));
                ui.rounded(px, py, 1.5, ph, 0.0, color::hex(0xffffff, 40));
                for (i, (label, action)) in items.into_iter().enumerate() {
                    let y = py + 6.0 + i as f32 * row;
                    if i > 0 {
                        ui.rounded(px + 24.0, y, pw - 48.0, 1.0, 0.0, color::hex(0xffffff, 24));
                    }
                    ui.text(
                        Face::Medium,
                        19.0,
                        px + 22.0,
                        y + 15.0,
                        color::hex(0xffffff, 255),
                        label,
                    );
                    if self.home_menu {
                        self.targets.insert(0, ([px, y, pw, row], action));
                    }
                }
            }
            let tab = [w - 30.0 - pw * p, h / 2.0 - 34.0, 30.0, 68.0];
            // Flush with the right edge: the right corners run off screen.
            ui.rounded(
                tab[0],
                tab[1],
                tab[2] + 24.0 * (1.0 - p),
                tab[3],
                14.0,
                color::hex(0x000000, 150),
            );
            let arrow = if self.home_menu { ">" } else { "<" };
            let aw = ui.measure(Face::Medium, 28.0, arrow);
            ui.text(
                Face::Medium,
                28.0,
                tab[0] + (30.0 - aw) / 2.0,
                tab[1] + 18.0,
                color::hex(0xffffff, 200),
                arrow,
            );
            let hit = if self.home_menu {
                [0.0, 0.0, w, h]
            } else {
                [w - 40.0, tab[1] - 10.0, 40.0, tab[3] + 20.0]
            };
            if self.home_menu {
                self.targets.push((hit, Action::HomeMenu));
            } else {
                self.targets.insert(0, (hit, Action::HomeMenu));
            }
        }
        if self.session_worker.is_some() && !self.session_prefetch {
            // Pressing Play shows the loading screen at once; the account session and the skin
            // are prepared behind it, and the game opens when they are ready.
            self.targets.clear();
            self.list_targets.clear();
            self.list_view = None;
            super::sunset::draw(&mut ui, w, h);
            ui.rounded(0.0, 0.0, w, h, 0.0, color::hex(0x000000, 110));
            let title = "Horizon";
            let tw = ui.measure(Face::Medium, 64.0, title);
            ui.text(
                Face::Medium,
                64.0,
                (w - tw) / 2.0,
                h * 0.38,
                color::hex(0xffffff, 255),
                title,
            );
            let note = "Getting your game ready...";
            let nw = ui.measure(Face::Regular, 18.0, note);
            ui.text(
                Face::Regular,
                18.0,
                (w - nw) / 2.0,
                h * 0.38 + 64.0 * 1.45 + 8.0,
                color::hex(0xffffff, 190),
                note,
            );
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        crate::documents::draw_ui(&mut ui);
        ui.finish();

        if !r.render(&game::loading_frame(&b))? && !b.patches.is_empty() {
            // A suspended/lost surface skips the frame, including its glyph
            // uploads. Rebuild them next time instead of caching invisible text.
            horizon_client::ui_theme::reset_atlas();
        }
        Ok(())
    }
    pub(super) fn activate(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.session_worker.is_some() && self.chooser.is_none() && !self.exit_confirmation {
            return Ok(());
        }
        let Some(r) = &self.renderer else {
            return Ok(());
        };
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let ui = Ui::mobile(&self.gui.as_ref().unwrap().atlas, &mut b, (0.0, 0.0));
        let p = self.focus.and_then(|index| self.targets.get(index)).map_or(
            [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ],
            |([x, y, w, h], _)| [x + w / 2.0, y + h / 2.0],
        );
        let action = self
            .targets
            .iter()
            .find(|([x, y, w, h], _)| {
                (*x..*x + *w).contains(&p[0]) && (*y..*y + *h).contains(&p[1])
            })
            .map(|(_, a)| *a);
        if self.catalog.busy() || crate::documents::busy() {
            return Ok(());
        }
        self.feedback = action.map(|action| (action, std::time::Instant::now()));
        self.focus = None;
        if action == Some(Action::HomeMenu) {
            self.home_menu = !self.home_menu;
            self.menu_t = std::time::Instant::now();
            return Ok(());
        }
        if self.home_menu {
            self.home_menu = false;
            self.menu_t = std::time::Instant::now();
        }
        if let Some(pressed) = action {
            let audio = self.audio.get_or_insert_with(|| {
                let mut audio = horizon_client::audio::Audio::new(
                    &self.home.join(".local/horizon-client/26.3"),
                );
                let settings = horizon_client::settings::Settings::load(
                    &self
                        .catalog
                        .config
                        .game_home(&self.home)
                        .join("options.txt"),
                );
                audio.master = settings.volumes[0];
                let mut volumes = settings.volumes;
                volumes[0] = 1.0;
                horizon_client::settings::set_source_volumes(volumes);
                audio
            });
            // Changing the page has a soft swish of its own; every other button a tap.
            if matches!(pressed, Action::Page(_)) {
                audio.launcher_swipe();
            } else {
                audio.launcher_click();
            }
        }
        match action {
            Some(Action::ConfirmExit) => {
                events.exit();
            }
            Some(Action::Immersive) => {
                self.catalog.config.screen.immersive ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::Cutout) => {
                self.catalog.config.screen.avoid_cutout ^= true;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
            }
            Some(Action::Aspect) => self.open_chooser(chooser::Kind::Aspect),
            Some(Action::ChooseAspect(aspect)) => {
                self.catalog.config.screen.aspect = aspect;
                self.catalog.config.save(&self.home)?;
                self.configure_screen();
                self.chooser = None;
            }
            Some(Action::ResetSettings) => {
                // Back to the defaults of the settings on this page; the player's name, server,
                // profile and add-on sources stay.
                let old = std::mem::take(&mut self.catalog.config);
                let config = &mut self.catalog.config;
                config.name = old.name;
                config.server = old.server;
                config.profile = old.profile;
                config.proxy = old.proxy;
                config.microsoft_client_id = old.microsoft_client_id;
                config.mod_source = old.mod_source;
                config.shader_source = old.shader_source;
                self.catalog.config.save(&self.home)?;
                self.catalog.status = "Settings reset to defaults.".into();
            }
            Some(Action::CacheBudget) => self.open_chooser(chooser::Kind::CacheBudget),
            Some(Action::ChooseCacheBudget(mib)) => {
                self.catalog.config.cache_budget_mib = mib.min(1024);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Disposable cache budget saved. Applies on next Play; not a total RAM or VRAM limit.".into();
            }
            Some(Action::ToggleBundled(id)) => {
                let game = self.catalog.config.game_home(&self.home);
                let mut list = horizon_client::mod_list::ModList::load(&game);
                let enabled = list.enabled.contains(id) && !list.disabled.contains(id);
                if enabled {
                    list.enabled.remove(id);
                    list.disabled.insert(id.into());
                } else {
                    list.disabled.remove(id);
                    list.enabled.insert(id.into());
                }
                std::fs::create_dir_all(&game)?;
                std::fs::write(game.join(horizon_client::mod_list::FILE), list.serialize())?;
                self.catalog.refresh(&self.home);
                self.catalog.status = format!(
                    "Bundled {id} {}. Applies on Play; settings are in Options > Mods.",
                    if enabled { "disabled" } else { "enabled" }
                );
            }
            Some(Action::GameFiles) => {
                self.catalog.status = if crate::platform::open_documents(crate::document_store::ROOT) {
                    "Game files opened. Worlds, mods, packs, shaders and reports are available here."
                } else { "Open system Files > Horizon Android." }.into();
            }
            Some(Action::ImportWorld) => {
                crate::platform::hide();
                let home = self.catalog.config.game_home(&self.home);
                if let Err(e) = crate::documents::begin_world(home) {
                    self.catalog.status = e;
                }
            }
            Some(Action::WorldFiles) => {
                let id = crate::document_store::folder_id(self.catalog.config.profile, "worlds");
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Copy a complete world folder to back up or import. Finish copying before Play; never replace an existing world silently."
                } else { "Open system Files > Horizon Android > Worlds. Copy complete world folders while the game is closed." }.into();
            }
            Some(Action::WorkerLimit) => self.open_chooser(chooser::Kind::WorkerLimit),
            Some(Action::ChooseWorkerLimit(limit)) => {
                self.catalog.config.worker_limit = limit.min(64);
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                self.catalog.status = "Chunk worker ceiling saved. Applies on the next Play; other engine threads are unchanged.".into();
            }
            Some(Action::DisplayHz) => self.open_chooser(chooser::Kind::DisplayHz),
            Some(Action::ChooseDisplayHz(hz)) => {
                self.catalog.config.device.display_hz = hz;
                self.catalog.config.device.sanitize();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
                self.chooser = None;
            }
            Some(Action::PerformanceHints) => {
                self.catalog.config.device.performance_hints ^= true;
                self.catalog.status="ADPF gives Android CPU frame-work hints on supported devices. It does not cap FPS or guarantee a speedup.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }
            Some(Action::GpuProfile) => {
                self.catalog.config.device.gpu_profile ^= true;
                self.catalog.status="Detailed GPU profiling records diagnostics and has overhead. Reports remain on this device.".into();
                self.catalog.config.save(&self.home)?;
                crate::device_runtime::configure(self.catalog.config.device);
            }

            Some(Action::ModSource(origin)) => {
                self.catalog.config.mod_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::ShaderSource(origin)) => {
                self.catalog.config.shader_source = origin;
                self.catalog.config.save(&self.home)?;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::Import(kind)) => {
                crate::platform::hide();
                let home = self.catalog.config.game_home(&self.home);
                if let Err(e) = crate::documents::begin(kind, home) {
                    self.catalog.status = e;
                }
            }
            Some(Action::ToggleSection(i)) => {
                self.catalog.sections[i] = !self.catalog.sections[i];
                self.catalog.scroll = 0.0;
            }
            Some(Action::ToggleLocal(i)) => self.catalog.toggle_local(&self.home, i),
            Some(Action::DeleteLocal(i)) => {
                self.catalog.pending_delete = self.catalog.local.get(i).cloned();
                if let Some(item) = &self.catalog.pending_delete {
                    self.chooser = Some(super::chooser::Chooser::confirm_delete(item.file.clone()));
                }
            }
            Some(Action::BuiltInInfo) => {
                self.catalog.status =
                    "Built into the app: it can be switched ON or OFF, not removed.".into();
            }
            Some(Action::ConfirmDeleteLocal) => {
                self.catalog.remove_local(&self.home);
                self.chooser = None;
            }
            Some(Action::Repository) => {
                self.catalog.status = if crate::platform::open_repository() {
                    "Repository opened in browser."
                } else {
                    "No browser available to open the repository."
                }
                .into();
            }
            Some(Action::Account) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.login.clear_note();
                self.login.providers = true;
                self.login.offline = false;
                self.login.ely = false;
                self.login.choosing = false;
                self.login.visible = true;
            }
            Some(Action::OfflineAccount | Action::MicrosoftAccount) => {
                self.login.providers = false;
                self.catalog.scroll = 0.0;
                self.login.offline = matches!(action, Some(Action::OfflineAccount));
                self.login.ely = false;
                self.login.choosing = false;
                self.login.visible = true;
            }
            Some(Action::SavedAccounts) => {
                self.login.providers = false;
                self.login.refresh_status();
                self.login.choosing = true;
                self.login.ely = false;
            }
            Some(Action::SavedPage(next)) => {
                if next && self.login.account_offset + 5 < self.login.saved_accounts.len() {
                    self.login.account_offset += 5;
                } else if !next {
                    self.login.account_offset = self.login.account_offset.saturating_sub(5);
                }
            }
            Some(Action::SelectAccount(index)) => {
                let mut accounts = crate::account::Accounts::load();
                if let Some(account) = self.login.saved_accounts.get(index)
                    && accounts.accounts.iter().any(|a| a.key() == account.key())
                {
                    accounts.selected = Some(account.key());
                    accounts.save()?;
                }
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.refresh_status();
                self.wardrobe.open(
                    &self.catalog.config.name,
                    &self.catalog.config.microsoft_client_id,
                );
            }
            Some(Action::SelectOffline) => {
                self.catalog.config.save(&self.home)?;
                self.catalog.editing = None;
                crate::platform::hide();
                self.login.hide();
                let mut accounts = crate::account::Accounts::load();
                accounts.selected = None;
                accounts.save()?;
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.choosing = false;
                self.login.refresh_status();
                self.wardrobe.open(
                    &self.catalog.config.name,
                    &self.catalog.config.microsoft_client_id,
                );
            }
            Some(Action::ElySignIn) => {
                self.catalog.scroll = 0.0;
                self.login.providers = false;
                self.login.close();
                self.login.clear_note();
                self.login.ely = true;
                self.login.visible = true;
            }
            Some(Action::SubmitEly) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                if !self.login.pending() {
                    self.login.start_ely();
                }
            }
            Some(Action::CloseLogin) => {
                self.login.hide();
                crate::platform::hide();
                self.catalog.editing = None;
                self.refresh_wardrobe_account();
            }
            Some(Action::DeviceSignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login
                    .start_device(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::CopyLoginCode) => self.login.copy_code(),
            Some(Action::CancelLogin) => {
                self.login.close();
                self.login.visible = true;
            }
            Some(Action::ReviewApp) => self.login.review_app(),
            Some(Action::SignIn) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.start(&self.catalog.config.microsoft_client_id);
            }
            Some(Action::SignOut) => {
                self.session_ready = false;
                self.prepared_session = None;
                self.session_worker = None;
                self.login.sign_out();
                self.refresh_wardrobe_account();
            }
            Some(Action::LoginLink) => self.login.open_link(),
            Some(Action::Page(page)) => {
                if page == Page::Skins {
                    self.wardrobe.open(
                        &self.catalog.config.name,
                        &self.catalog.config.microsoft_client_id,
                    );
                }
                self.catalog.editing = None;
                if crate::platform::active() {
                    crate::platform::hide();
                }
                self.catalog.page = page;
                self.catalog.scroll = 0.0;
                self.error = None;
                self.catalog.offset = 0;
                self.catalog.browse(&self.home);
            }
            Some(Action::Play) => self.action(events)?,
            Some(Action::WardrobeKind(cape)) => {
                self.wardrobe.capes = cape;
                self.catalog.scroll = 0.0;
            }
            Some(Action::WardrobeSelect(index)) => self.wardrobe.select(index),
            Some(Action::WardrobeImport) => self.wardrobe.import(),
            Some(Action::WardrobeCloud) => self
                .wardrobe
                .cloud(&self.catalog.config.microsoft_client_id, None),
            Some(Action::WardrobeUpload) => self.wardrobe.cloud(
                &self.catalog.config.microsoft_client_id,
                Some(wardrobe::CloudAction::Upload),
            ),
            Some(Action::WardrobeActivateCape(index)) => self.wardrobe.cloud(
                &self.catalog.config.microsoft_client_id,
                Some(wardrobe::CloudAction::Cape(index)),
            ),
            Some(Action::WardrobeUseCloud) => self.wardrobe.use_cloud(),
            Some(Action::WardrobeModel) => self.wardrobe.model(),
            Some(Action::WardrobeRotate(direction)) => {
                self.wardrobe.camera.rotate(direction as f32 * 25.0, 0.0);
                self.wardrobe.preview();
            }
            Some(Action::WardrobeZoom(inward)) => {
                self.wardrobe
                    .camera
                    .zoom(if inward { 1.15 } else { 1.0 / 1.15 });
                self.wardrobe.preview();
            }
            Some(Action::WardrobeHideCape) => self.wardrobe.hide_cape(),
            Some(Action::Search) => {
                crate::platform::hide();
                self.catalog.offset = 0;
                self.catalog.search(&self.home);
            }
            Some(Action::FinishNameEdit) => {
                if self.catalog.editing == Some("name") {
                    self.refresh_wardrobe_account();
                }
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::PasteInto(name)) => {
                self.catalog.editing = Some(name);
                {
                    let text: String = crate::platform::clipboard_get()
                        .unwrap_or_default()
                        .chars()
                        .filter(|c| !c.is_control())
                        .take(256)
                        .collect();
                    if !text.is_empty() {
                        self.edit_text(Output::Text(text));
                        // The system editor holds the whole value, so later typing and
                        // deleting continue from what was pasted.
                        if let Some(value) = self.login.field(name) {
                            if name == "ely_password" || name == "ely_otp" {
                                crate::platform::show_secret(value);
                            } else {
                                crate::platform::show(value);
                            }
                        } else if name == "name" {
                            crate::platform::show(&self.catalog.config.name);
                        }
                    }
                }
            }
            Some(Action::Field(name)) => {
                if let Some(value) = self.login.field(name) {
                    self.catalog.editing = Some(name);
                    if name == "ely_password" || name == "ely_otp" {
                        crate::platform::show_secret(value);
                    } else {
                        crate::platform::show(value);
                    }
                    return Ok(());
                }
                if name == "render_scale" {
                    self.catalog.render_scale_text =
                        self.catalog.config.render_scale.percent().to_string();
                    self.chooser = None;
                }
                self.catalog.editing = Some(name);
                let initial = match name {
                    "name" => &self.catalog.config.name,
                    "server" => &self.catalog.config.server,
                    "render_scale" => &self.catalog.render_scale_text,
                    "client_id" => &self.catalog.config.microsoft_client_id,
                    _ => &self.catalog.query,
                };
                crate::platform::show(initial);
            }
            Some(Action::Install(i)) => self.catalog.install(&self.home, i),
            Some(Action::Next) => {
                self.catalog.offset += 12;
                self.catalog.search(&self.home);
            }
            Some(Action::Previous) => {
                self.catalog.offset = self.catalog.offset.saturating_sub(12);
                self.catalog.search(&self.home);
            }
            Some(Action::Profile) => self.open_chooser(chooser::Kind::Profile),
            Some(Action::Renderer) => self.open_chooser(chooser::Kind::Renderer),
            Some(Action::RenderScale) => self.open_chooser(chooser::Kind::RenderScale),
            Some(Action::ChooseRenderScale(scale)) => {
                self.catalog.config.render_scale = scale;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::display::set_scale(scale);
            }
            Some(Action::Orientation) => self.open_chooser(chooser::Kind::Orientation),
            Some(Action::InputHost) => self.open_chooser(chooser::Kind::InputHost),
            Some(Action::ChooseInputHost(mode)) => {
                self.catalog.config.input_host = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::configure_input_mode(mode);
            }
            Some(Action::ChooseOrientation(mode)) => {
                self.catalog.config.orientation = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::platform::set_orientation(mode);
            }
            Some(Action::ChooseRenderer(mode)) => {
                self.catalog.config.renderer = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
            }
            Some(Action::ChooseProfile(profile)) => {
                std::fs::create_dir_all(if profile == 0 {
                    self.home.clone()
                } else {
                    self.home
                        .join("profiles")
                        .join(format!("profile-{profile}"))
                })?;
                self.catalog.config.profile = profile;
                self.catalog.config.save(&self.home)?;
                self.catalog.refresh(&self.home);
                self.chooser = None;
            }
            Some(Action::Dismiss) => {
                self.exit_confirmation = false;
                self.chooser = None;
                if self.session_worker.is_none() {
                    self.session_ready = false;
                    self.prepared_session = None;
                    self.session_worker = None;
                }
            }
            Some(Action::Logs) => {
                self.catalog.status = crate::diagnostics::summary(&self.home);
                if let Ok(error) = std::fs::read_to_string(self.home.join("android-error.txt")) {
                    self.catalog.status = format!(
                        "Last handled error: {}. {}",
                        crate::diagnostics::redact(&error),
                        self.catalog.status
                    );
                }
            }
            Some(Action::ExportDiagnostics) if self.diagnostic_export.is_none() => {
                let home = self.home.clone();
                let (sender, receiver) = mpsc::channel();
                self.diagnostic_export = Some(receiver);
                self.catalog.status =
                    "Preparing local diagnostic ZIP. No upload; review before sharing.".into();
                std::thread::spawn(move || {
                    let result = crate::diagnostics::export(&home).map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    });
                    let _ = sender.send(result);
                });
            }
            Some(Action::ExportDiagnostics) => {}
            Some(Action::RecordPerformance) => {
                self.catalog.config.record_performance = !self.catalog.config.record_performance;
                if !self.catalog.config.record_performance {
                    crate::performance::stop();
                }
                self.catalog.config.save(&self.home)?;
                self.catalog.status = if self.catalog.config.record_performance {
                    "Recording starts on Play. Detailed timings add overhead. Return here to save the report."
                } else { "Performance recording off." }.into();
            }
            Some(Action::ExportPerformance) => {
                self.catalog.status = match crate::performance::export(&self.home) {
                    Ok(()) => "Reports opened in Files. Copy or share a report from there.".into(),
                    Err(e) => e,
                };
            }
            // The side menu is handled above, before this match.
            Some(Action::HomeMenu) | None => {}
        }
        Ok(())
    }
    pub(super) fn ui_density(&self) -> f32 {
        let (Some(r), Some(gui)) = (&self.renderer, &self.gui) else {
            return 1.0;
        };
        let mut batch = HudBatch::new([r.config.width, r.config.height]);
        Ui::mobile(&gui.atlas, &mut batch, (0.0, 0.0)).density
    }

    pub(super) fn touch_event(&mut self, events: &ActiveEventLoop, t: Touch) -> Result<()> {
        if self.catalog.page == Page::Skins
            && !self.login.visible
            && self.wardrobe.gesture(&t, self.ui_density())
        {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return Ok(());
        }
        self.focus = None;
        self.pending_focus = None;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.cursor = (t.location.x, t.location.y);
        match t.phase {
            TouchPhase::Started => {
                self.drag = Some(super::gesture::Drag::new(t.location.x, t.location.y));
            }
            TouchPhase::Moved => {
                let density = self.ui_density() as f64;
                if let Some(drag) = &mut self.drag {
                    let delta = drag.update(t.location.x, t.location.y, density);
                    if drag.moved {
                        if let Some(chooser) = &mut self.chooser {
                            chooser.scroll = (chooser.scroll + delta).max(0.0);
                        } else if self
                            .list_view
                            .is_some_and(|view| drag.origin_y() / density >= view[0] as f64)
                        {
                            self.catalog.scroll = (self.catalog.scroll + delta).max(0.0);
                        }
                    }
                }
            }
            TouchPhase::Ended => {
                if self.drag.take().is_some_and(|drag| !drag.moved) {
                    self.activate(events)?;
                }
            }
            TouchPhase::Cancelled => {
                self.drag = None;
            }
        }
        Ok(())
    }
    pub(super) fn type_key(&mut self, key: KeyEvent) {
        if key.physical_key == PhysicalKey::Code(KeyCode::Escape)
            || key.logical_key
                == winit::keyboard::Key::Named(winit::keyboard::NamedKey::BrowserBack)
        {
            if !key.repeat {
                self.request_back();
            }
            return;
        }
        if self.catalog.editing.is_none() {
            return;
        }
        let out = match key.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => Some(Output::Backspace),
            PhysicalKey::Code(KeyCode::Enter) => {
                crate::platform::hide();
                Some(Output::Enter)
            }
            _ => key.text.map(|s| Output::Text(s.into())),
        };
        if let Some(out) = out {
            self.edit_text(out);
        }
    }
}
