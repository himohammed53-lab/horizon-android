//! Native account panel; authentication is shared with the desktop launcher.
use super::pages::{Action, SWITCH_ON, button as page_button, button_as as page_button_as};
use crate::account::{self, Account, Accounts, Secrets, SignIn, SignInState};
use horizon_client::ui_theme::{Face, Ui, color};

pub(super) struct Login {
    pub choosing: bool,
    pub providers: bool,
    pub offline: bool,
    pub saved_accounts: Vec<Account>,
    pub account_offset: usize,
    pub ely: bool,
    pub username: String,
    pub password: String,
    pub otp: String,
    /// Ely.by asked for the two-factor code: the field shows, and the password waits for it.
    pub otp_needed: bool,
    pub visible: bool,
    pub signed_in: bool,
    pub profile: Option<String>,
    /// Which field the page last jumped to, with the room it had.
    snapped: std::cell::Cell<(Option<&'static str>, i32)>,
    task: Option<SignIn>,
    auto_open_device: bool,
    state: Option<SignInState>,
    message: String,
}
/// Wide buttons are drawn a little narrower than their touch area; widen the area so that the
/// drawn plate spans the dialog evenly instead of leaning left.
fn fit(rect: [f32; 4]) -> [f32; 4] {
    rect
}
fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
) {
    page_button(ui, targets, fit(rect), label, action, accent);
}
fn button_as(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
    tint: Option<(u32, u32)>,
) {
    page_button_as(ui, targets, fit(rect), label, action, accent, tint);
}

impl Login {
    pub fn new() -> Self {
        let mut login = Self {
            choosing: false,
            providers: false,
            offline: false,
            saved_accounts: vec![],
            account_offset: 0,
            ely: false,
            username: String::new(),
            password: String::new(),
            otp: String::new(),
            otp_needed: false,
            visible: false,
            signed_in: false,
            profile: None,
            snapped: std::cell::Cell::new((None, 0)),
            task: None,
            auto_open_device: false,
            state: None,
            message: String::new(),
        };
        login.refresh_status();
        login
    }
    pub(super) fn refresh_status(&mut self) {
        let accounts = Accounts::load();
        let account = accounts.selected().cloned();
        self.saved_accounts = accounts.accounts;
        self.profile = account.as_ref().map(|a| a.name.clone());
        self.signed_in = account.is_some_and(|a| {
            account::store()
                .get(&a.key())
                .ok()
                .flatten()
                .is_some_and(|s| serde_json::from_str::<Secrets>(&s).is_ok())
        });
    }
    #[allow(dead_code)]
    pub fn status(&self, offline_name: &str) -> String {
        match (&self.profile, self.signed_in) {
            (Some(name), true) => format!("Signed in: {name}"),
            (Some(name), false) => format!("Sign in required: {name}"),
            _ if self.pending() => "Signing in...".into(),
            _ if !offline_name.trim().is_empty() => format!("Offline: {offline_name}"),
            _ => "Offline / Sign in".into(),
        }
    }

    pub fn pending(&self) -> bool {
        self.task.is_some()
    }
    pub fn hide(&mut self) {
        self.password.clear();
        self.otp.clear();
        self.otp_needed = false;
        self.visible = false;
    }
    pub fn close(&mut self) {
        self.password.clear();
        self.otp.clear();
        self.otp_needed = false;
        self.auto_open_device = false;
        if let Some(task) = self.task.take() {
            task.cancel();
        }
        self.state = None;
        self.visible = false;
    }
    pub fn start(&mut self, id: &str) {
        self.start_device(id);
        self.auto_open_device = true;
    }
    pub fn start_device(&mut self, id: &str) {
        self.close();
        self.ely = false;
        self.choosing = false;
        self.visible = true;
        let id = id.trim();
        if !account::legacy_client(id)
            && (id.len() != 36
                || !id.chars().enumerate().all(|(i, c)| {
                    if [8, 13, 18, 23].contains(&i) {
                        c == '-'
                    } else {
                        c.is_ascii_hexdigit()
                    }
                }))
        {
            self.message =
                "Register our Microsoft public client application first, then enter its client ID."
                    .into();
            return;
        }
        self.message.clear();
        self.task = Some(SignIn::start_with_client_id(Some(id.into())));
        self.state = Some(SignInState::Starting);
    }
    /// Reports completion so views can switch their provider-scoped selection,
    /// including when the account panel was closed during browser sign-in.
    pub fn poll(&mut self) -> bool {
        let mut completed = false;
        if let Some(task) = &self.task {
            self.state = Some(task.state());
            if let Some(SignInState::Done(result)) = &self.state {
                self.message = match result {
                    Ok(a) => format!("Signed in as {}", a.name),
                    Err(e) => e.clone(),
                };
                // Only an account protected by two-factor asks for the code, and only now: the
                // password stays for the second try; anything else forgets it.
                self.otp_needed = matches!(result, Err(e) if e.contains("two-factor code and"));
                if !self.otp_needed {
                    self.password.clear();
                }
                self.task = None;
                self.refresh_status();
                completed = true;
            }
        }
        if self.auto_open_device && matches!(self.state, Some(SignInState::Code(_))) {
            self.auto_open_device = false;
            self.open_link();
        }
        completed
    }
    pub fn start_ely(&mut self) {
        if let Some(task) = self.task.take() {
            task.cancel();
        }
        self.message.clear();
        self.task = Some(SignIn::start_ely(
            self.username.clone(),
            self.password.clone(),
            std::mem::take(&mut self.otp),
        ));
        self.state = Some(SignInState::Starting);
    }
    pub fn field(&self, name: &str) -> Option<&str> {
        match name {
            "ely_username" => Some(&self.username),
            "ely_password" => Some(&self.password),
            "ely_otp" => Some(&self.otp),
            _ => None,
        }
    }
    pub fn edit(&mut self, field: Option<&str>, output: &horizon_client::osk::Output) -> bool {
        let value = match field {
            Some("ely_username") => &mut self.username,
            Some("ely_password") => &mut self.password,
            Some("ely_otp") => &mut self.otp,
            _ => return false,
        };
        match output {
            horizon_client::osk::Output::Text(text) if value.len() + text.len() <= 1024 => {
                value.push_str(text);
            }
            horizon_client::osk::Output::Backspace => {
                value.pop();
            }
            _ => {}
        }
        true
    }
    pub fn sign_out(&mut self) {
        self.close();
        self.visible = true;
        let mut accounts = Accounts::load();
        if let Some(a) = accounts.selected().cloned() {
            self.message = match accounts
                .remove_when_idle(account::store(), &a.key())
                .and_then(|_| accounts.save())
            {
                Ok(()) => "Signed out. Offline play is available.".into(),
                Err(e) => e,
            };
        }
        self.refresh_status();
    }
    pub fn copy_code(&mut self) {
        if let Some(SignInState::Code(code)) = &self.state {
            self.message = if crate::platform::copy_login_code(&code.user_code) {
                "Code copied. Paste it on the Microsoft page.".into()
            } else {
                "Could not copy the code. Try again.".into()
            };
        }
    }
    pub fn open_link(&mut self) {
        if let Some(SignInState::Code(code)) = &self.state {
            let mut url = url::Url::parse(&code.verification_uri).ok();
            if let Some(uri) = &mut url {
                uri.query_pairs_mut().append_pair("otc", &code.user_code);
            }
            let url = url
                .map(String::from)
                .unwrap_or_else(|| code.verification_uri.clone());
            self.copy_code();
            if let Err(e) = crate::platform::open_login_url(&url) {
                self.message = e;
            }
        }
    }
    pub fn review_app(&mut self) {
        if let Err(e) = crate::platform::open_login_url("https://aka.ms/mce-reviewappid") {
            self.message = e;
        }
    }
    pub fn session(id: &str) -> Result<Option<(Account, Secrets)>, String> {
        account::session::selected_session(id.trim())
    }
    pub fn draw(
        &self,
        ui: &mut Ui,
        targets: &mut Vec<([f32; 4], Action)>,
        offline_name: &str,
        editing: Option<&'static str>,
        scroll: f32,
    ) -> Option<([f32; 3], f32)> {
        targets.clear();
        let w = (ui.w - 32.0).min(720.0);
        let h = (ui.h - 24.0).min(550.0);
        let x = (ui.w - w) / 2.0;
        let y = (ui.h - h) / 2.0;
        // A thin dark-grey outline (the search field's grey) so the dialog does not blend in.
        ui.rounded(
            x - 2.0,
            y - 2.0,
            w + 4.0,
            h + 4.0,
            if super::pages::GLASS { 22.0 } else { 10.0 },
            color::hex(0x5a5a5a, 255),
        );
        ui.rounded(
            x,
            y,
            w,
            h,
            if super::pages::GLASS { 20.0 } else { 8.0 },
            color::BG,
        );
        if self.providers {
            ui.text(
                Face::Medium,
                24.0,
                x + 20.0,
                y + 20.0,
                color::TEXT,
                "Accounts",
            );
            let half = (w - 52.0) / 2.0;
            for (index, (label, action)) in [
                ("Offline (name only)", Action::OfflineAccount),
                ("Microsoft account", Action::MicrosoftAccount),
                ("Ely.by", Action::ElySignIn),
                ("Saved accounts", Action::SavedAccounts),
            ]
            .into_iter()
            .enumerate()
            {
                button_as(
                    ui,
                    targets,
                    [
                        x + 20.0 + (index % 2) as f32 * (half + 12.0),
                        y + 58.0 + (index / 2) as f32 * 54.0,
                        half,
                        46.0,
                    ],
                    label,
                    action,
                    true,
                    Some((SWITCH_ON, SWITCH_ON)),
                );
            }
            // Under the buttons: every account that is signed in, with its service's mark.
            let top = y + 170.0;
            ui.text(
                Face::Regular,
                13.0,
                x + 20.0,
                top,
                color::MUTED,
                "Signed-in accounts",
            );
            let room = (((y + h - 62.0) - (top + 20.0)) / 54.0).floor().max(0.0) as usize;
            let signed: Vec<&Account> = self
                .saved_accounts
                .iter()
                .filter(|a| {
                    account::store()
                        .get(&a.key())
                        .ok()
                        .flatten()
                        .is_some_and(|s| serde_json::from_str::<Secrets>(&s).is_ok())
                })
                .collect();
            if signed.is_empty() {
                ui.text(
                    Face::Regular,
                    14.0,
                    x + 20.0,
                    top + 26.0,
                    color::TEXT,
                    "Not signed in to any service.",
                );
            }
            for (row, account) in signed.iter().take(room.max(1)).enumerate() {
                let card_y = top + 20.0 + row as f32 * 54.0;
                let current = self.profile.as_deref() == Some(account.name.as_str());
                ui.rounded(
                    x + 20.0,
                    card_y - 1.0,
                    w - 40.0,
                    50.0,
                    8.0,
                    if current {
                        color::ACCENT
                    } else {
                        color::hex(0x5a5a5a, 255)
                    },
                );
                ui.rounded(x + 21.0, card_y, w - 42.0, 48.0, 7.0, color::RAISED);
                let (lx, ly) = (x + 34.0, card_y + 10.0);
                let ely = account.provider == account::Provider::Ely;
                if ely {
                    ui.rounded(lx, ly, 28.0, 28.0, 6.0, color::hex(0x3c8a3a, 255));
                    ui.text(
                        Face::Medium,
                        20.0,
                        lx + 8.0,
                        ly + 3.0,
                        color::hex(0xffffff, 255),
                        "E",
                    );
                } else {
                    for (i, rgb) in [0xf25022, 0x7fba00, 0x00a4ef, 0xffb900]
                        .into_iter()
                        .enumerate()
                    {
                        ui.rounded(
                            lx + (i % 2) as f32 * 15.0,
                            ly + (i / 2) as f32 * 15.0,
                            13.0,
                            13.0,
                            1.0,
                            color::hex(rgb, 255),
                        );
                    }
                }
                ui.text(
                    Face::Medium,
                    16.0,
                    x + 76.0,
                    card_y + 6.0,
                    color::TEXT,
                    &ui.fit_text(Face::Medium, 16.0, &account.name, w - 220.0),
                );
                ui.text(
                    Face::Regular,
                    13.0,
                    x + 76.0,
                    card_y + 28.0,
                    color::MUTED,
                    if ely { "Ely.by" } else { "Microsoft" },
                );
                ui.text(
                    Face::Medium,
                    14.0,
                    x + w - 120.0,
                    card_y + 15.0,
                    color::ACCENT,
                    if current { "Signed in" } else { "Saved" },
                );
            }
            button(
                ui,
                targets,
                [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
                "Close",
                Action::CloseLogin,
                false,
            );
            return None;
        }
        if self.choosing {
            ui.text(
                Face::Medium,
                24.0,
                x + 20.0,
                y + 20.0,
                color::TEXT,
                "Choose account",
            );
            button(
                ui,
                targets,
                [x + 20.0, y + 62.0, w - 40.0, 36.0],
                &format!("Offline: {offline_name}"),
                Action::SelectOffline,
                false,
            );
            for (index, account) in self
                .saved_accounts
                .iter()
                .enumerate()
                .skip(self.account_offset)
                .take(5)
            {
                let label = format!(
                    "{} · {}",
                    account.name,
                    if account.provider == account::Provider::Ely {
                        "Ely.by"
                    } else {
                        "Microsoft"
                    }
                );
                button(
                    ui,
                    targets,
                    [
                        x + 20.0,
                        y + 110.0 + (index - self.account_offset) as f32 * 42.0,
                        w - 40.0,
                        36.0,
                    ],
                    &label,
                    Action::SelectAccount(index),
                    false,
                );
            }
            button(
                ui,
                targets,
                [x + 20.0, y + h - 96.0, (w - 48.0) * 0.5, 32.0],
                "Previous",
                Action::SavedPage(false),
                false,
            );
            button(
                ui,
                targets,
                [x + w * 0.5 + 4.0, y + h - 96.0, (w - 48.0) * 0.5, 32.0],
                "Next",
                Action::SavedPage(true),
                false,
            );
            button(
                ui,
                targets,
                [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
                "Back",
                Action::Account,
                false,
            );
            return None;
        }
        if self.ely || self.offline {
            // One scrolling page: the panel never rearranges when the keyboard opens; its
            // viewport just gets shorter and the page scrolls (drag it), jumping to the field
            // being typed in.
            if self.ely {
                ui.rounded(
                    x + 20.0,
                    y + 14.0,
                    36.0,
                    36.0,
                    6.0,
                    color::hex(0x3c8a3a, 255),
                );
                ui.text(
                    Face::Medium,
                    26.0,
                    x + 29.0,
                    y + 17.0,
                    color::hex(0xffffff, 255),
                    "E",
                );
                ui.text(
                    Face::Medium,
                    24.0,
                    x + 68.0,
                    y + 20.0,
                    color::TEXT,
                    "Ely.by account",
                );
            } else {
                ui.text(
                    Face::Medium,
                    24.0,
                    x + 20.0,
                    y + 20.0,
                    color::TEXT,
                    "Offline player name",
                );
            }
            let (top, bottom) = (y + 64.0, y + h - 10.0);
            let view_h = bottom - top;
            let paste_w = 104.0;
            let field_w = w - 40.0 - paste_w - 8.0;
            // (label, field name, text shown)
            let mut fields: Vec<(&str, &'static str, String)> = Vec::new();
            if self.ely {
                fields.push(("Email", "ely_username", self.username.clone()));
                fields.push((
                    "Password",
                    "ely_password",
                    "*".repeat(self.password.chars().count().min(40)),
                ));
                if self.otp_needed {
                    fields.push((
                        "Two-factor code",
                        "ely_otp",
                        "*".repeat(self.otp.chars().count().min(40)),
                    ));
                }
            } else {
                fields.push((
                    "Player name (maximum 16 characters)",
                    "name",
                    if offline_name.is_empty() {
                        String::new()
                    } else {
                        offline_name.to_owned()
                    },
                ));
            }
            const NOTE: f32 = 28.0;
            const ROW: f32 = 92.0;
            let buttons_at = NOTE + fields.len() as f32 * ROW + 6.0;
            let total = buttons_at + 62.0 + 56.0 + 8.0;
            let maximum = (total - view_h).max(0.0);
            let mut scroll = scroll.clamp(0.0, maximum);
            // Bring the field being typed in into view when it starts and when the keyboard
            // changes the room; a hand scroll in between is left alone.
            let marker = (editing, view_h.round() as i32);
            if self.snapped.get() != marker {
                self.snapped.set(marker);
                if let Some(index) = fields.iter().position(|f| Some(f.1) == editing) {
                    let first = NOTE + index as f32 * ROW;
                    if first < scroll || first + ROW - 12.0 > scroll + view_h {
                        scroll = (first - 4.0).clamp(0.0, maximum);
                    }
                }
            }
            let visible = |at: f32, height: f32| at >= 0.0 && at + height <= view_h;
            let at = |c: f32| top + c - scroll;
            // The note: the last result, else what the page is for.
            if visible(0.0 - scroll, 22.0) {
                let note = if self.message.is_empty() {
                    if self.ely {
                        "Credentials go only to authserver.ely.by."
                    } else {
                        "Used for play without an account."
                    }
                } else {
                    &self.message
                };
                let note = ui.fit_text(Face::Regular, 14.0, note, w - 40.0);
                ui.text(
                    Face::Regular,
                    14.0,
                    x + 20.0,
                    at(0.0) + 2.0,
                    if self.message.is_empty() {
                        color::MUTED
                    } else {
                        color::TEXT
                    },
                    &note,
                );
            }
            for (index, (label, name, shown)) in fields.iter().enumerate() {
                let c = NOTE + index as f32 * ROW;
                if !visible(c - scroll, ROW - 12.0) {
                    continue;
                }
                ui.text(Face::Medium, 15.0, x + 20.0, at(c), color::MUTED, label);
                button(
                    ui,
                    targets,
                    [x + 20.0, at(c) + 24.0, field_w, 56.0],
                    shown,
                    Action::Field(*name),
                    false,
                );
                button(
                    ui,
                    targets,
                    [x + 28.0 + field_w, at(c) + 24.0, paste_w, 56.0],
                    "Paste",
                    Action::PasteInto(*name),
                    false,
                );
            }
            let (main_label, main_action) = if self.ely {
                (
                    if self.pending() {
                        "Signing in..."
                    } else {
                        "Sign in to Ely.by"
                    },
                    Action::SubmitEly,
                )
            } else {
                ("Use offline account", Action::SelectOffline)
            };
            if visible(buttons_at - scroll, 56.0) {
                button(
                    ui,
                    targets,
                    [x + 20.0, at(buttons_at), w - 40.0, 56.0],
                    main_label,
                    main_action,
                    true,
                );
            }
            let half = (w - 52.0) / 2.0;
            let second = buttons_at + 66.0;
            if visible(second - scroll, 50.0) {
                button(
                    ui,
                    targets,
                    [x + 20.0, at(second), half, 50.0],
                    "Accounts",
                    Action::Account,
                    false,
                );
                button(
                    ui,
                    targets,
                    [x + 32.0 + half, at(second), half, 50.0],
                    "Done",
                    Action::CloseLogin,
                    false,
                );
            }
            // A scroll bar when the page is longer than its room.
            if maximum > 0.0 {
                let thumb = (view_h * view_h / total).max(24.0);
                let at_y = top + (view_h - thumb) * (scroll / maximum);
                ui.rounded(x + w - 8.0, at_y, 4.0, thumb, 2.0, color::MUTED);
            }
            return Some(([top, bottom, maximum], scroll));
        }

        ui.text(
            Face::Medium,
            24.0,
            x + 20.0,
            y + 18.0,
            color::TEXT,
            "Microsoft account",
        );
        ui.text(
            Face::Medium,
            18.0,
            x + 20.0,
            y + 58.0,
            if self.signed_in {
                color::ACCENT
            } else {
                color::MUTED
            },
            &self.status(offline_name),
        );
        if let Some(SignInState::Code(code)) = &self.state {
            button(
                ui,
                targets,
                [x + 20.0, y + 96.0, w - 40.0, 48.0],
                &format!("{}    /    Copy code", code.user_code),
                Action::CopyLoginCode,
                false,
            );
        } else {
            ui.text(
                Face::Regular,
                16.0,
                x + 20.0,
                y + 104.0,
                color::TEXT,
                if self.signed_in {
                    "Account saved securely. Sessions renew automatically."
                } else {
                    "Your password is entered in Microsoft's browser."
                },
            );
        }
        let text = match &self.state {
            Some(SignInState::Starting) => "Requesting a sign-in code...",
            _ => &self.message,
        };
        let limit = ((w - 40.0) / 7.5).max(20.0) as usize;
        let mut line = String::new();
        let mut row = 0;
        for word in text.split_whitespace() {
            if line.len() + word.len() + 1 > limit {
                ui.text(
                    Face::Regular,
                    14.0,
                    x + 20.0,
                    y + 166.0 + row as f32 * 20.0,
                    color::TEXT,
                    &line,
                );
                row += 1;
                line.clear();
                if row >= 5 {
                    break;
                }
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if row < 5 {
            ui.text(
                Face::Regular,
                14.0,
                x + 20.0,
                y + 166.0 + row as f32 * 20.0,
                color::TEXT,
                &line,
            );
        }
        let half = (w - 52.0) / 2.0;
        if self.message.contains("not approved by Mojang") {
            button(
                ui,
                targets,
                [x + 20.0, y + h - 183.0, w - 40.0, 30.0],
                "Request Minecraft API approval",
                Action::ReviewApp,
                false,
            );
        }
        let (label, action) = if matches!(self.state, Some(SignInState::Code(_))) {
            ("Copy code and open browser", Action::LoginLink)
        } else if self.pending() {
            ("Restart browser sign-in", Action::SignIn)
        } else {
            (
                if self.signed_in {
                    "Switch Microsoft account"
                } else {
                    "Sign in with browser"
                },
                Action::SignIn,
            )
        };
        button(
            ui,
            targets,
            [x + 20.0, y + h - 148.0, w - 40.0, 42.0],
            label,
            action,
            true,
        );
        button(
            ui,
            targets,
            [x + 20.0, y + h - 98.0, half, 38.0],
            "Use a device code instead",
            Action::DeviceSignIn,
            false,
        );
        button(
            ui,
            targets,
            [x + 32.0 + half, y + h - 98.0, half, 38.0],
            if self.pending() {
                "Cancel sign-in"
            } else {
                if self.profile.is_some() {
                    "Sign out"
                } else {
                    "Close"
                }
            },
            if self.pending() {
                Action::CancelLogin
            } else {
                if self.profile.is_some() {
                    Action::SignOut
                } else {
                    Action::CloseLogin
                }
            },
            false,
        );
        button(
            ui,
            targets,
            [x + 20.0, y + h - 52.0, w - 40.0, 36.0],
            if self.pending() {
                "Back (sign-in continues)"
            } else {
                "Done"
            },
            Action::CloseLogin,
            false,
        );
        None
    }
}
impl Drop for Login {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.cancel();
        }
    }
}
