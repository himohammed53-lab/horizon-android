//! Horizon mod: brighter light. `/gamma <1-16>` sets how bright the world is
//! drawn (1 is the Brightness option's own "Bright"), `/gamma off` gives the
//! player's option back. It starts at 8.
use horizon_mod_sdk::{Mod, Registration, chat, render};

const DEFAULT: f32 = 8.0;

struct Gamma {
    value: f32,
    /// The value was sent to the game.
    sent: bool,
}

impl Gamma {
    fn send(&mut self) {
        render::set_brightness(self.value);
        self.sent = true;
    }
}

impl Mod for Gamma {
    fn init() -> Result<(Self, Registration), String> {
        let state = Gamma {
            value: DEFAULT,
            sent: false,
        };
        Ok((state, Registration::new().command("gamma").on_tick()))
    }

    fn tick(&mut self, _tick: u64) {
        if !self.sent {
            self.send();
        }
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let arg = args.trim();
        if arg.is_empty() {
            chat::show(&format!("Gamma is {}", self.value));
            return Ok(None);
        }
        if arg == "off" {
            self.value = 0.0;
            self.send();
            return Ok(Some("Gamma is off: your own brightness".into()));
        }
        let value: f32 = arg
            .parse()
            .map_err(|_| "Use /gamma 1 to 16, or /gamma off".to_string())?;
        if !(1.0..=16.0).contains(&value) {
            return Err("Use /gamma 1 to 16, or /gamma off".into());
        }
        self.value = value;
        self.send();
        Ok(Some(format!("Gamma is {value}")))
    }
}

horizon_mod_sdk::export_mod!(Gamma);
