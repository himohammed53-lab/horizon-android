//! Explicit, scrollable selections. Values never cycle on repeated button taps.
use super::{
    Launcher,
    pages::{Action, button},
};
use crate::catalog::RendererMode;
use horizon_client::ui_theme::{Face, Ui, color};

pub(super) enum Kind {
    Aspect,
    DisplayHz,
    CacheBudget,
    WorkerLimit,
    Renderer,
    RenderScale,
    Profile,
    Orientation,
    InputHost,
}
pub(super) struct Chooser {
    title: &'static str,
    help: &'static str,
    device: Option<String>,
    choices: Vec<(String, String, Action, bool)>,
    pub scroll: f32,
}
impl Launcher {
    pub(super) fn open_chooser(&mut self, kind: Kind) {
        crate::platform::hide();
        self.catalog.editing = None;
        let device = if matches!(kind, Kind::Renderer) {
            self.renderer.as_ref().map(|r| {
                format!(
                    "GPU ({}): {} / {} {}",
                    gpu_family(&r.info.name, r.info.vendor).unwrap_or("other"),
                    r.info.name,
                    r.info.driver,
                    r.info.driver_info
                )
            })
        } else {
            None
        };
        let c = &self.catalog.config;
        let (title, help, choices) = match kind {
            Kind::Aspect => (
                "Aspect ratio",
                "Fits the entire picture without stretching. Touch coordinates follow the picture. Portrait swaps width and height.",
                [
                    crate::display::Aspect::Native,
                    crate::display::Aspect::Wide,
                    crate::display::Aspect::Classic,
                    crate::display::Aspect::Square,
                ]
                .into_iter()
                .map(|a| {
                    (
                        a.label().into(),
                        String::new(),
                        Action::ChooseAspect(a),
                        c.screen.aspect == a,
                    )
                })
                .collect(),
            ),
            Kind::CacheBudget => (
                "Reusable RAM cache",
                "Decoded sounds (80%) and empty mesh buffers (20%). Recreated at the same quality. Smaller budgets can increase CPU work. Not total RAM/VRAM. Applies on next Play.",
                [0, 16, 32, 64, 128, 256]
                    .into_iter()
                    .map(|mib| {
                        (
                            if mib == 0 {
                                "Engine default (80 MiB)".into()
                            } else {
                                format!("{mib} MiB")
                            },
                            String::new(),
                            Action::ChooseCacheBudget(mib),
                            c.cache_budget_mib == mib,
                        )
                    })
                    .collect(),
            ),
            Kind::WorkerLimit => (
                "Chunk worker limit",
                "Ceiling for generation and meshing workers, not all app threads. Lower values can reduce CPU contention but slow loading. Applies on next Play.",
                (0..=64u32)
                    .map(|limit| {
                        (
                            if limit == 0 {
                                "No additional limit".into()
                            } else {
                                format!("{limit} worker(s)")
                            },
                            String::new(),
                            Action::ChooseWorkerLimit(limit),
                            c.worker_limit == limit,
                        )
                    })
                    .collect(),
            ),
            Kind::DisplayHz => (
                "Display refresh rate",
                "An Android display preference, not a game FPS cap. Unsupported rates may be ignored.",
                crate::device_preferences::DISPLAY_RATES
                    .into_iter()
                    .map(|hz| {
                        (
                            if hz == 0 {
                                "System default".into()
                            } else {
                                format!("{hz} Hz")
                            },
                            String::new(),
                            Action::ChooseDisplayHz(hz),
                            c.device.display_hz == hz,
                        )
                    })
                    .collect(),
            ),
            Kind::RenderScale => {
                let mut choices: Vec<_> = crate::display::RenderScale::PRESETS
                    .into_iter()
                    .map(|percent| {
                        (
                            format!("{percent}%"),
                            if percent == 100 {
                                "Native resolution".into()
                            } else {
                                "Lower resolution for the whole picture".into()
                            },
                            Action::ChooseRenderScale(crate::display::RenderScale::new(percent)),
                            c.render_scale.percent() == percent,
                        )
                    })
                    .collect();
                choices.push((
                    "Custom percentage...".into(),
                    "Enter a value from 25 to 100".into(),
                    Action::Field("render_scale"),
                    false,
                ));
                (
                    "Client resolution",
                    "Small screens retain a usable menu size.",
                    choices,
                )
            }
            Kind::InputHost => (
                "Input mode",
                "Mouse mode uses a physical keyboard. Touch mode keeps the Android keyboard.",
                crate::catalog::InputHost::ALL
                    .into_iter()
                    .map(|mode| {
                        (
                            mode.label().into(),
                            String::new(),
                            Action::ChooseInputHost(mode),
                            c.input_host == mode,
                        )
                    })
                    .collect(),
            ),
            Kind::Orientation => (
                "Screen orientation",
                "Applies to the launcher and game. Does not change touch bindings.",
                crate::catalog::Orientation::ALL
                    .into_iter()
                    .map(|mode| {
                        (
                            mode.label().into(),
                            String::new(),
                            Action::ChooseOrientation(mode),
                            c.orientation == mode,
                        )
                    })
                    .collect(),
            ),
            Kind::Renderer => {
                // The suggestion follows the GPU: tested on an Adreno 725 (fine) and a Mali-G76
                // (about 25 FPS at full resolution, hence the weak GPU mode).
                let family = self
                    .renderer
                    .as_ref()
                    .and_then(|r| gpu_family(&r.info.name, r.info.vendor));
                let weak = family == Some("PowerVR")
                    || self
                        .renderer
                        .as_ref()
                        .and_then(|r| super::device_report::gpu_tier(&r.info.name))
                        .is_some();
                let note = |mode: RendererMode| -> String {
                    match (family, mode) {
                        (Some("Mali"), RendererMode::WeakGpu) if weak => {
                            " Suggested for your Mali GPU (a Mali-G76 ran about 25 FPS at full resolution).".into()
                        }
                        (_, RendererMode::WeakGpu) if weak => " Suggested for your GPU.".into(),
                        (Some("Adreno"), RendererMode::VulkanCompatibility) if !weak => {
                            " Suggested for your Adreno GPU.".into()
                        }
                        (Some("Mali"), RendererMode::VulkanCompatibility) if !weak => {
                            " Suggested for your Mali GPU (not tested on this one yet).".into()
                        }
                        (Some("Mali"), RendererMode::OpenGl) => {
                            " Much slower than Vulkan on the Mali-G76 tested; use only if Vulkan fails.".into()
                        }
                        (Some("PowerVR"), RendererMode::OpenGl) => {
                            " Try this if Vulkan crashes or the screen stays black.".into()
                        }
                        _ => String::new(),
                    }
                };
                (
                    "Rendering mode",
                    "Uses the system GPU driver. Applies on the next game launch.",
                    vec![
                        (
                            "OpenGL ES / experimental".into(),
                            format!(
                                "For older GPUs. Requires ES 3.0+ and sufficient driver limits.{}",
                                note(RendererMode::OpenGl)
                            ),
                            Action::ChooseRenderer(RendererMode::OpenGl),
                            c.renderer == RendererMode::OpenGl,
                        ),
                        (
                            "Vulkan / GPU batching".into(),
                            format!(
                                "Groups terrain draws and enables supported GPU culling.{}",
                                note(RendererMode::Vulkan)
                            ),
                            Action::ChooseRenderer(RendererMode::Vulkan),
                            c.renderer == RendererMode::Vulkan,
                        ),
                        (
                            "Vulkan / compatibility".into(),
                            format!(
                                "Direct terrain draws for drivers with missing or transparent blocks.{}",
                                note(RendererMode::VulkanCompatibility)
                            ),
                            Action::ChooseRenderer(RendererMode::VulkanCompatibility),
                            c.renderer == RendererMode::VulkanCompatibility,
                        ),
                        (
                            "Vulkan / weak GPU (Mali)".into(),
                            format!(
                                "For weak GPUs (Mali, older Adreno, PowerVR): Vulkan as in compatibility, with a lower resolution (50-67%) and the Performance graphics preset. Choosing it sets them once; change them in Settings.{}",
                                note(RendererMode::WeakGpu)
                            ),
                            Action::ChooseRenderer(RendererMode::WeakGpu),
                            c.renderer == RendererMode::WeakGpu,
                        ),
                    ],
                )
            }
            Kind::Profile => {
                let mut ids = vec![0, c.profile];
                ids.extend(
                    std::fs::read_dir(self.home.join("profiles"))
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter(|e| e.path().is_dir())
                        .filter_map(|e| {
                            e.file_name()
                                .to_str()?
                                .strip_prefix("profile-")?
                                .parse::<u32>()
                                .ok()
                        }),
                );
                ids.sort_unstable();
                ids.dedup();
                let next = ids.last().copied().unwrap_or(0).checked_add(1);
                let mut choices: Vec<_> = ids
                    .into_iter()
                    .map(|id| {
                        (
                            format!("Profile {id}"),
                            if id == 0 {
                                "Default profile".into()
                            } else {
                                "Separate saves, mods and packs".into()
                            },
                            Action::ChooseProfile(id),
                            id == c.profile,
                        )
                    })
                    .collect();
                if let Some(id) = next {
                    choices.push((
                        "Create profile".into(),
                        format!("Create Profile {id}"),
                        Action::ChooseProfile(id),
                        false,
                    ));
                }
                (
                    "Choose profile",
                    "Each profile keeps its own game settings and saves.",
                    choices,
                )
            }
        };
        self.chooser = Some(Chooser {
            title,
            help,
            choices,
            device,
            scroll: 0.0,
        });
    }
}
impl Chooser {
    pub(super) fn confirm_delete(name: String) -> Self {
        Self {
            title: "Remove?",
            help: "Deletes only the selected file from this profile. This cannot be undone.",
            device: Some(name),
            choices: vec![
                (
                    "Remove".into(),
                    String::new(),
                    Action::ConfirmDeleteLocal,
                    false,
                ),
                ("Cancel".into(), String::new(), Action::Dismiss, true),
            ],
            scroll: 0.0,
        }
    }
    pub(super) fn navigation(&self, width: f32) -> Vec<([f32; 4], Action)> {
        self.choices
            .iter()
            .enumerate()
            .map(|(i, (_, _, action, _))| {
                (
                    [
                        32.0,
                        137.0 + i as f32 * 84.0 - self.scroll,
                        width - 64.0,
                        76.0,
                    ],
                    *action,
                )
            })
            .collect()
    }
    pub(super) fn viewport(&self, height: f32) -> [f32; 3] {
        [
            137.0,
            height - 100.0,
            (self.choices.len() as f32 * 84.0 - (height - 237.0)).max(0.0),
        ]
    }
    pub fn draw(&mut self, ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>) {
        let (w, h) = (ui.w, ui.h);
        targets.clear();
        ui.rounded(0.0, 0.0, w, h, 0.0, color::BG);
        ui.text(Face::Medium, 28.0, 32.0, 35.0, color::TEXT, self.title);
        ui.text(Face::Regular, 15.0, 32.0, 85.0, color::MUTED, self.help);
        if let Some(device) = &self.device {
            ui.text(Face::Regular, 13.0, 32.0, 111.0, color::MUTED, device);
        }
        let top = 137.0;
        let bottom = h - 100.0;
        self.scroll = self.scroll.clamp(
            0.0,
            (self.choices.len() as f32 * 84.0 - (bottom - top)).max(0.0),
        );
        for (i, (label, detail, action, selected)) in self.choices.iter().enumerate() {
            let y = top + i as f32 * 84.0 - self.scroll;
            if y < top || y + 76.0 > bottom {
                continue;
            }
            ui.rounded(34.0, y + 10.0, 22.0, 22.0, 11.0, color::TEXT);
            ui.rounded(36.0, y + 12.0, 18.0, 18.0, 9.0, color::BG);
            if *selected {
                ui.rounded(40.0, y + 16.0, 10.0, 10.0, 5.0, color::TEXT);
            }
            ui.text(Face::Medium, 19.0, 80.0, y + 8.0, color::TEXT, label);
            ui.text(Face::Regular, 14.0, 80.0, y + 40.0, color::MUTED, detail);
            ui.rounded(32.0, y + 75.0, w - 64.0, 1.0, 0.0, color::BORDER);
            targets.push(([32.0, y, w - 64.0, 76.0], *action));
        }
        button(
            ui,
            targets,
            [32.0, h - 78.0, 160.0, 44.0],
            "Cancel",
            Action::Dismiss,
            false,
        );
    }
}

/// The GPU family of an Android phone: Qualcomm's Adreno, Arm's Mali (also in MediaTek and
/// Samsung chips) or Imagination's PowerVR, from the Vulkan vendor id or the adapter name.
fn gpu_family(name: &str, vendor: u32) -> Option<&'static str> {
    let name = name.to_ascii_lowercase();
    if vendor == 0x5143 || name.contains("adreno") {
        Some("Adreno")
    } else if vendor == 0x13b5 || name.contains("mali") {
        Some("Mali")
    } else if vendor == 0x1010 || name.contains("powervr") || name.contains("img ") {
        Some("PowerVR")
    } else {
        None
    }
}
