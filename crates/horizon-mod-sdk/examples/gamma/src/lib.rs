//! Horizon mod: brighter light, so caves and the night are lit. Its page in
//! Options > Mods has two buttons: Gamma ON or OFF, and the brightness it
//! cycles through (1 is the Brightness option's own "Bright"). `/gamma <1-16>`
//! and `/gamma off` do the same from chat. The choice is kept in the mod's
//! storage.
use horizon_mod_sdk::{
    Mod, Registration, chat, render, storage,
    ui::{self, ButtonWidget, Widget},
};

/// The brightness the button cycles through.
const LEVELS: [u8; 5] = [1, 2, 4, 8, 16];
const DEFAULT: u8 = 8;
const ON_BUTTON: u32 = 1;
const LEVEL_BUTTON: u32 = 2;

struct Gamma {
    on: bool,
    level: u8,
    /// The stored choice was sent to the game.
    sent: bool,
}

fn stored(key: &str) -> Option<String> {
    storage::get(key).and_then(|v| String::from_utf8(v).ok())
}

impl Gamma {
    /// Tells the game the brightness in effect (0 gives the player's own).
    fn send(&self) {
        render::set_brightness(if self.on { f32::from(self.level) } else { 0.0 });
    }
    fn page(&self) {
        let on = if self.on { "ON" } else { "OFF" };
        ui::set_settings(&[
            Widget::Button(ButtonWidget {
                id: ON_BUTTON,
                label: format!("Gamma: {on}"),
            }),
            Widget::Button(ButtonWidget {
                id: LEVEL_BUTTON,
                label: format!("Brightness: {}x", self.level),
            }),
        ]);
    }
    fn save(&self) {
        let _ = storage::set("on", if self.on { &b"true"[..] } else { &b"false"[..] });
        let _ = storage::set("level", self.level.to_string().as_bytes());
    }
    fn changed(&self) {
        self.send();
        self.save();
        self.page();
    }
}

impl Mod for Gamma {
    fn init() -> Result<(Self, Registration), String> {
        let level = stored("level")
            .and_then(|v| v.trim().parse::<u8>().ok())
            .filter(|v| LEVELS.contains(v))
            .unwrap_or(DEFAULT);
        let on = stored("on").is_none_or(|v| v.trim() != "false");
        let gamma = Gamma {
            on,
            level,
            sent: false,
        };
        gamma.page();
        Ok((gamma, Registration::new().command("gamma").on_tick()))
    }

    /// The first tick sends the stored choice: the game keeps it until the
    /// next call.
    fn tick(&mut self, _tick: u64) {
        if !self.sent {
            self.send();
            self.sent = true;
        }
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let arg = args.trim();
        if arg.is_empty() {
            chat::show(&format!(
                "Gamma is {} at {}x",
                if self.on { "on" } else { "off" },
                self.level
            ));
            return Ok(None);
        }
        if arg == "off" {
            self.on = false;
            self.changed();
            return Ok(Some("Gamma is off: your own brightness".into()));
        }
        let value: u8 = arg
            .parse::<u8>()
            .ok()
            .filter(|v| (1..=16).contains(v))
            .ok_or("Use /gamma 1 to 16, or /gamma off")?;
        self.on = true;
        self.level = value;
        self.changed();
        Ok(Some(format!("Gamma is {value}x")))
    }

    fn ui_select(&mut self, id: u32, _row: u32) {
        match id {
            ON_BUTTON => self.on = !self.on,
            LEVEL_BUTTON => {
                let i = LEVELS.iter().position(|&l| l == self.level).unwrap_or(0);
                self.level = LEVELS[(i + 1) % LEVELS.len()];
            }
            _ => return,
        }
        self.changed();
    }

    fn ui_settings_opened(&mut self) {
        self.page();
    }
}

horizon_mod_sdk::export_mod!(Gamma);
