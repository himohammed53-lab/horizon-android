//! SAF selection, bounded background imports and explicit collision choices.
use horizon_client::{
    Result,
    content_files::{self, Kind},
    hud::{HudAtlas, HudBatch, rgba},
    ui_theme::{Face, Ui, color},
};
use jni::{
    JNIEnv,
    objects::{JObject, JString},
};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

struct Request {
    kind: Kind,
    /// A world archive to unpack into the saves folder.
    world: bool,
    appearance: Option<crate::skins::library::Kind>,
    home: PathBuf,
    cache: PathBuf,
}
struct Conflict {
    kind: Kind,
    staged: PathBuf,
    target: PathBuf,
}
#[derive(Clone)]
pub struct Notice {
    pub text: String,
    pub changed: bool,
    pub kind: Option<Kind>,
}
#[derive(Default)]
struct State {
    request: Option<Request>,
    working: bool,
    conflict: Option<Conflict>,
    notice: Option<Notice>,
    hits: Vec<([f32; 4], Choice)>,
    toast: Option<(String, std::time::Instant)>,
    cursor: (f32, f32),
}
#[derive(Clone, Copy)]
pub enum Choice {
    KeepBoth,
    Replace,
    Cancel,
}
static STATE: Mutex<State> = Mutex::new(State {
    request: None,
    working: false,
    conflict: None,
    notice: None,
    hits: Vec::new(),
    toast: None,
    cursor: (0.0, 0.0),
});
static CANCEL: AtomicBool = AtomicBool::new(false);
pub fn busy() -> bool {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.request.is_some() || s.working || s.conflict.is_some()
}
pub fn conflict() -> bool {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .conflict
        .is_some()
}
fn notice(text: impl Into<String>, changed: bool) {
    let text = text.into();
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.toast = Some((text.clone(), std::time::Instant::now()));
    s.notice = Some(Notice {
        text,
        changed,
        kind: None,
    });
}
fn imported(text: impl Into<String>, kind: Kind) {
    let text = text.into();
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.toast = Some((text.clone(), std::time::Instant::now()));
    s.notice = Some(Notice {
        text,
        changed: true,
        kind: Some(kind),
    });
}
pub fn poll() -> Option<Notice> {
    if crate::document_provider::take_changes() {
        return Some(Notice {
            text: "Content files changed. Reload packs to apply.".into(),
            changed: true,
            kind: None,
        });
    }
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .notice
        .take()
}
pub fn begin(kind: Kind, home: PathBuf) -> std::result::Result<(), String> {
    if busy() {
        return Err("Finish or cancel the current import first".into());
    }
    log::info!("World import: opening the picker");
    CANCEL.store(false, Ordering::Relaxed);
    crate::platform::hide();
    let cache = crate::platform::cache_dir()
        .ok_or("Could not access the Android import cache")?
        .join("content-imports");
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = Some(Request {
        kind,
        world: false,
        appearance: None,
        home,
        cache,
    });
    if !crate::native_ime::pick_document() {
        picker_failed("Android document picker is unavailable");
        return Err("Android document picker is unavailable".into());
    }
    notice(format!("Choose a {} file", kind.label()), false);
    Ok(())
}
pub fn picker_failed(error: &str) {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = None;
    notice(error, false);
}
pub fn begin_world(home: PathBuf) -> std::result::Result<(), String> {
    if busy() {
        return Err("Finish or cancel the current import first".into());
    }
    CANCEL.store(false, Ordering::Relaxed);
    crate::platform::hide();
    let cache = crate::platform::cache_dir()
        .ok_or("Could not access the Android import cache")?
        .join("world-imports");
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = Some(Request {
        kind: Kind::Packs,
        world: true,
        appearance: None,
        home,
        cache,
    });
    if !crate::native_ime::pick_document() {
        picker_failed("Android document picker is unavailable");
        return Err("Android document picker is unavailable".into());
    }
    notice("Choose a world ZIP file", false);
    Ok(())
}
pub fn begin_appearance(
    home: PathBuf,
    kind: crate::skins::library::Kind,
) -> std::result::Result<(), String> {
    if busy() {
        return Err("Finish or cancel the current import first".into());
    }
    CANCEL.store(false, Ordering::Relaxed);
    crate::platform::hide();
    let cache = crate::platform::cache_dir()
        .ok_or("Could not access Android's import cache")?
        .join("appearance-imports");
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = Some(Request {
        kind: Kind::Packs,
        world: false,
        appearance: Some(kind),
        home,
        cache,
    });
    if !crate::native_ime::pick_document() {
        picker_failed("Android document picker is unavailable");
        return Err("Android document picker is unavailable".into());
    }
    notice("Choose a skin or cape PNG", false);
    Ok(())
}
/// Called by the approved Activity result bridge, never from a game tick.
/// Keep a conventional JNI export as well as explicit registration: Android
/// may restore a picker result before the IME bridge has been initialized.
#[unsafe(export_name = "Java_dev_horizon_android_HorizonActivity_nativeDocumentResult")]
pub extern "system" fn document_result(
    mut env: JNIEnv<'_>,
    activity: JObject<'_>,
    request: i32,
    result: i32,
    uri: JString<'_>,
) {
    log::info!("Document result: request {request}, result {result}");
    if request != 4101 {
        return;
    }
    let pending = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .request
        .take();
    let Some(pending) = pending else {
        notice("Import interrupted. Choose the file again.", false);
        return;
    };
    if result != -1 || uri.is_null() {
        notice("Import cancelled", false);
        return;
    }
    let setup = (|| -> jni::errors::Result<_> {
        let text: String = env.get_string(&uri)?.into();
        let activity = env.new_global_ref(activity)?;
        Ok((env.get_java_vm()?, activity, text))
    })();
    let Ok((vm, activity, uri)) = setup else {
        let _ = env.exception_clear();
        notice("Could not read the selected document", false);
        return;
    };
    if !uri.starts_with("content://") || uri.len() > 8192 {
        notice("Unsupported document URI", false);
        return;
    }
    STATE.lock().unwrap_or_else(|e| e.into_inner()).working = true;
    let spawned = std::thread::Builder::new()
        .name("Content import".into())
        .spawn(move || {
            let result = (|| -> Result<()> {
                let mut env = vm.attach_current_thread()?;
                let fallback = if pending.kind == Kind::Mods {
                    "import.hmod"
                } else {
                    "import.zip"
                };
                let opened =
                    crate::document_bridge::open(&mut env, activity.as_obj(), &uri, fallback);
                if opened.is_err() {
                    let _ = env.exception_clear();
                }
                let (input, name) = opened.map_err(|e| e.to_string())?;
                stage(pending, input, name)
            })();
            STATE.lock().unwrap_or_else(|e| e.into_inner()).working = false;
            if let Err(error) = result {
                log::warn!("Import failed: {error}");
                notice(format!("Import failed: {error}"), false);
            }
        });
    if let Err(error) = spawned {
        STATE.lock().unwrap_or_else(|e| e.into_inner()).working = false;
        notice(format!("Import failed: {error}"), false);
    }
}
fn stage(request: Request, mut input: std::fs::File, name: String) -> Result<()> {
    if request.world {
        return stage_world(request, input, name);
    }
    if let Some(kind) = request.appearance {
        use std::io::Read;
        if !name.to_ascii_lowercase().ends_with(".png") {
            return Err("Choose a PNG skin or cape".into());
        }
        let mut bytes = Vec::new();
        input
            .take(crate::skins::MAX_IMPORT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if CANCEL.load(Ordering::Relaxed) {
            return Err("Import cancelled".into());
        }
        let mut library = crate::skins::library::Library::load(&request.home)?;
        library.import(&request.home, &name, kind, &bytes)?;
        notice(
            "PNG added to the local gallery. Select it to apply locally.",
            false,
        );
        return Ok(());
    }
    if !content_files::plain_name(&name) || !request.kind.accepts(&name) {
        return Err("Selected file has the wrong extension for this folder".into());
    }
    let dir = request.home.join(request.kind.folder());
    std::fs::create_dir_all(&dir)?;
    if std::fs::symlink_metadata(&dir)?.file_type().is_symlink() {
        return Err("Refusing a redirected content directory".into());
    }
    std::fs::create_dir_all(&request.cache)?;
    if std::fs::symlink_metadata(&request.cache)?
        .file_type()
        .is_symlink()
    {
        return Err("Refusing a redirected import cache".into());
    }
    for entry in std::fs::read_dir(&request.cache)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(".import-") && name.ends_with(".part") && entry.file_type()?.is_file() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let staged = request.cache.join(format!(".import-{stamp}.part"));
    let result = (|| -> Result<()> {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)?;
        content_files::copy_bounded(&mut input, &mut output, |bytes| {
            notice(
                format!("Importing {}: {} KiB", request.kind.label(), bytes / 1024),
                false,
            );
            !CANCEL.load(Ordering::Relaxed)
        })?;
        output.sync_all()?;
        drop(output);
        notice("Checking archive...", false);
        let native_shader = content_files::validate(&staged, request.kind, &name, || {
            CANCEL.load(Ordering::Relaxed)
        })?;
        let name = if request.kind == Kind::Mods {
            format!("{}.hmod", horizon_mods::Package::read(&staged)?.manifest.id)
        } else if native_shader || name.to_ascii_lowercase().ends_with(".hshader") {
            let pack = horizon_shader_api::Pack::read(&staged)?;
            horizon_shader_api::check(&pack).map_err(|e| format!("Invalid shader pack: {e:?}"))?;
            let mut path = PathBuf::from(name);
            path.set_extension("hshader");
            path.to_string_lossy().into_owned()
        } else {
            name
        };
        if !content_files::plain_name(&name) {
            return Err("Package has an invalid ID or filename".into());
        }
        if CANCEL.load(Ordering::Relaxed) {
            return Err("Import cancelled".into());
        }
        let target = dir.join(name);
        if target.exists() {
            STATE.lock().unwrap_or_else(|e| e.into_inner()).conflict = Some(Conflict {
                kind: request.kind,
                staged: staged.clone(),
                target,
            });
            notice(
                "File already exists: choose Keep both, Replace or Cancel",
                false,
            );
        } else {
            std::fs::rename(&staged, &target)?;
            imported(
                format!(
                    "Imported {} into {}. Enable it in game settings.",
                    target.file_name().unwrap_or_default().to_string_lossy(),
                    request.kind.label()
                ),
                request.kind,
            );
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(staged);
    }
    result
}
const WORLD_INPUT: u64 = 3 << 30;
const WORLD_EXPANDED: u64 = 6 << 30;
const WORLD_ENTRIES: usize = 200_000;

/// Unpack a zipped Java world into `saves/` beside the others. Every path is checked first,
/// nothing is ever overwritten (a taken name gets " (2)"), and the world appears only once whole.
fn stage_world(request: Request, mut input: std::fs::File, name: String) -> Result<()> {
    use std::io::{Read, Write};
    if !content_files::plain_name(&name) || !name.to_ascii_lowercase().ends_with(".zip") {
        return Err("Choose a ZIP file containing your world folder".into());
    }
    let saves = request.home.join("saves");
    std::fs::create_dir_all(&saves)?;
    if std::fs::symlink_metadata(&saves)?.file_type().is_symlink() {
        return Err("Refusing a redirected saves directory".into());
    }
    std::fs::create_dir_all(&request.cache)?;
    if std::fs::symlink_metadata(&request.cache)?
        .file_type()
        .is_symlink()
    {
        return Err("Refusing a redirected import cache".into());
    }
    for entry in std::fs::read_dir(&request.cache)?.flatten() {
        if entry.file_name().to_string_lossy().starts_with(".world-")
            && entry.file_type()?.is_file()
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let staged = request.cache.join(format!(".world-{stamp}.part"));
    let temp = saves.join(format!(".import-{stamp}.tmp"));
    let result = (|| -> Result<()> {
        // 1. Read the picked file in place when Android lets us seek in it: no second copy of
        //    the ZIP is made. Otherwise copy it into the app's cache first (deleted afterwards).
        let direct = input
            .try_clone()
            .ok()
            .and_then(|file| zip::ZipArchive::new(file).ok());
        let mut zip = match direct {
            Some(zip) => zip,
            None => {
                use std::io::Seek;
                input.rewind()?;
                {
                    let mut output = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&staged)?;
                    let mut buffer = vec![0u8; 1 << 18];
                    let mut total = 0u64;
                    loop {
                        if CANCEL.load(Ordering::Relaxed) {
                            return Err("Import cancelled".into());
                        }
                        let n = input.read(&mut buffer)?;
                        if n == 0 {
                            break;
                        }
                        total += n as u64;
                        if total > WORLD_INPUT {
                            return Err("World ZIP is larger than 3 GiB".into());
                        }
                        output.write_all(&buffer[..n])?;
                        notice(format!("Copying world: {} MiB", total >> 20), false);
                    }
                    output.sync_all()?;
                }
                zip::ZipArchive::new(std::fs::File::open(&staged)?)?
            }
        };
        // 2. Check every entry and find the folder holding level.dat.
        notice("Checking world...", false);
        if zip.len() > WORLD_ENTRIES {
            return Err("Archive contains too many files".into());
        }
        let mut names = std::collections::HashSet::new();
        let mut total = 0u64;
        let mut level: Option<String> = None;
        for i in 0..zip.len() {
            let entry = zip.by_index(i)?;
            let raw = entry.name().to_owned();
            if raw.is_empty()
                || raw.len() > 1024
                || raw.chars().any(char::is_control)
                || raw.contains(['\\', ':', '\0'])
                || raw.starts_with('/')
                || raw.split('/').any(|p| p == ".." || p == ".")
                || !names.insert(raw.clone())
                || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                return Err("Archive contains an unsafe or duplicate path".into());
            }
            total = total.saturating_add(entry.size());
            if total > WORLD_EXPANDED {
                return Err("World is larger than 6 GiB unpacked".into());
            }
            let parts: Vec<&str> = raw.split('/').collect();
            if !entry.is_dir()
                && parts.last() == Some(&"level.dat")
                && parts.len() <= 3
                && parts[0] != "__MACOSX"
                && level
                    .as_ref()
                    .is_none_or(|l| l.split('/').count() > parts.len())
            {
                level = Some(raw);
            }
        }
        let level = level.ok_or("No level.dat found: this is not a Java world ZIP")?;
        let prefix = level.strip_suffix("level.dat").unwrap_or("").to_owned();
        // 3. Pick a free folder name.
        let mut base = prefix
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                name.trim_end_matches(".zip")
                    .trim_end_matches(".ZIP")
                    .to_owned()
            });
        base = base.chars().filter(|c| !c.is_control()).take(60).collect();
        if !content_files::plain_name(&base) {
            base = "Imported world".into();
        }
        let mut folder = base.clone();
        let mut n = 2;
        while saves.join(&folder).exists() {
            folder = format!("{base} ({n})");
            n += 1;
            if n > 1000 {
                return Err("Too many worlds with this name".into());
            }
        }
        // 4. Unpack beside the others under a temporary name, then rename it into place.
        std::fs::create_dir(&temp)?;
        let mut done = 0u64;
        let mut buffer = vec![0u8; 1 << 18];
        for i in 0..zip.len() {
            if CANCEL.load(Ordering::Relaxed) {
                return Err("Import cancelled".into());
            }
            let mut entry = zip.by_index(i)?;
            let raw = entry.name().to_owned();
            let Some(rest) = raw.strip_prefix(prefix.as_str()) else {
                continue;
            };
            if rest.is_empty() || raw.starts_with("__MACOSX/") {
                continue;
            }
            let target = temp.join(rest);
            if entry.is_dir() {
                std::fs::create_dir_all(&target)?;
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            loop {
                let n = entry.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                done += n as u64;
                if done > WORLD_EXPANDED {
                    return Err("World is larger than 6 GiB unpacked".into());
                }
                output.write_all(&buffer[..n])?;
            }
            if i % 32 == 0 {
                notice(format!("Unpacking world: {} MiB", done >> 20), false);
            }
        }
        if !temp.join("level.dat").is_file() {
            return Err("No level.dat found: this is not a Java world ZIP".into());
        }
        std::fs::rename(&temp, saves.join(&folder))?;
        notice(
            format!("World \"{folder}\" added. Open Play > Singleplayer to find it."),
            false,
        );
        Ok(())
    })();
    let _ = std::fs::remove_file(&staged);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&temp);
    }
    result
}
pub fn resolve(choice: Choice) {
    let c = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .conflict
        .take();
    let Some(mut c) = c else {
        return;
    };
    if matches!(choice, Choice::Cancel) {
        let _ = std::fs::remove_file(c.staged);
        notice("Import cancelled", false);
        return;
    }
    let result = (|| -> Result<()> {
        if matches!(choice, Choice::Replace)
            && std::fs::symlink_metadata(&c.target)
                .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err("Cannot replace a folder or symbolic link".into());
        }
        if matches!(choice, Choice::KeepBoth) {
            if c.kind == Kind::Mods {
                return Err(
                    "A mod ID cannot be installed twice; choose Replace to update it".into(),
                );
            }
            let mut stem = c
                .target
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            while stem.len() > 150 {
                stem.pop();
            }
            let ext = c
                .target
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let dir = c
                .target
                .parent()
                .ok_or("Invalid content folder")?
                .to_owned();
            for i in 1..=10000 {
                let candidate = dir.join(format!("{stem} ({i}).{ext}"));
                if !candidate.exists() {
                    c.target = candidate;
                    break;
                }
                if i == 10000 {
                    return Err("Too many filename conflicts".into());
                }
            }
        }
        std::fs::rename(&c.staged, &c.target)?;
        Ok(())
    })();
    match result {
        Ok(()) => imported(
            "Import completed. Enable it in Installed or game settings.",
            c.kind,
        ),
        Err(e) => {
            let _ = std::fs::remove_file(c.staged);
            notice(format!("Import failed: {e}"), false);
        }
    }
}
pub fn cancel() {
    CANCEL.store(true, Ordering::Relaxed);
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = None;
    if conflict() {
        resolve(Choice::Cancel);
    } else {
        notice("Import cancelled", false);
    }
}
pub fn tap(x: f32, y: f32) -> bool {
    if !busy() {
        return false;
    }
    let choice = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .hits
        .iter()
        .find(|(r, _)| x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3])
        .map(|(_, c)| *c);
    if let Some(choice) = choice {
        if matches!(choice, Choice::Cancel) {
            cancel();
        } else {
            resolve(choice);
        }
    }
    true
}
pub fn draw(batch: &mut HudBatch, atlas: &HudAtlas) {
    let mut ui = Ui::mobile(atlas, batch, (0.0, 0.0));
    draw_ui(&mut ui);
    ui.finish();
}
/// Launcher overlays must use its bundled font, including before game fonts exist.
pub fn draw_ui(ui: &mut Ui<'_>) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.hits.clear();
    if s.conflict.is_none() && !s.working {
        // Besides the status line (which not every page shows), the last result stays in a
        // bar at the bottom for a few seconds, so an import never ends in silence.
        if let Some((text, at)) = s.toast.as_ref()
            && at.elapsed() < std::time::Duration::from_secs(8)
        {
            let w = (ui.w - 24.0).min(640.0);
            let (x, y) = ((ui.w - w) / 2.0, ui.h - 64.0);
            ui.rounded(
                x - 1.0,
                y - 1.0,
                w + 2.0,
                50.0,
                14.0,
                color::hex(0xffffff, 110),
            );
            ui.rounded(x, y, w, 48.0, 13.0, color::hex(0x14161b, 245));
            let text = ui.fit_text(Face::Medium, 16.0, text, w - 28.0);
            ui.text(Face::Medium, 16.0, x + 14.0, y + 14.0, color::TEXT, &text);
        }
        return;
    }
    let (name, choices) = if let Some(c) = &s.conflict {
        (
            c.target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            if c.kind == Kind::Mods {
                vec![("Replace", Choice::Replace), ("Cancel", Choice::Cancel)]
            } else {
                vec![
                    ("Keep both", Choice::KeepBoth),
                    ("Replace", Choice::Replace),
                    ("Cancel", Choice::Cancel),
                ]
            },
        )
    } else {
        (
            s.toast.as_ref().map(|v| v.0.clone()).unwrap_or_default(),
            vec![("Cancel", Choice::Cancel)],
        )
    };
    let w = 440.0f32.min(ui.w - 24.0);
    let vertical = w < 320.0;
    let height = if vertical {
        92.0 + choices.len() as f32 * 44.0
    } else {
        144.0
    };
    let x = (ui.w - w) / 2.0;
    let y = ((ui.h - height) / 2.0).max(0.0);
    ui.rounded(0.0, 0.0, ui.w, ui.h, 0.0, rgba(0, 0, 0, 180));
    ui.rounded(x, y, w, height, 2.0, color::BG);
    ui.text(
        Face::Medium,
        20.0,
        x + 12.0,
        y + 12.0,
        color::TEXT,
        if s.conflict.is_some() {
            "File exists"
        } else {
            "Importing file"
        },
    );
    let name = ui.fit_text(Face::Regular, 16.0, &name, w - 24.0);
    ui.text(Face::Regular, 16.0, x + 12.0, y + 46.0, color::MUTED, &name);
    let width = (w - 24.0) / choices.len() as f32;
    for (i, (label, choice)) in choices.into_iter().enumerate() {
        let rect = if vertical {
            [x + 12.0, y + 86.0 + i as f32 * 44.0, w - 24.0, 38.0]
        } else {
            [x + 12.0 + i as f32 * width, y + 92.0, width - 4.0, 38.0]
        };
        ui.rounded(rect[0], rect[1], rect[2], rect[3], 2.0, color::RAISED);
        ui.text(
            Face::Medium,
            16.0,
            rect[0] + 8.0,
            rect[1] + 7.0,
            color::TEXT,
            label,
        );
        s.hits.push((
            [
                rect[0] * ui.density,
                rect[1] * ui.density,
                rect[2] * ui.density,
                rect[3] * ui.density,
            ],
            choice,
        ));
    }
}
pub fn handle_event(event: &winit::event::WindowEvent) -> bool {
    use winit::{
        event::{ElementState, MouseButton, TouchPhase, WindowEvent},
        keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    };
    if !busy() {
        return false;
    }
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            STATE.lock().unwrap_or_else(|e| e.into_inner()).cursor =
                (position.x as f32, position.y as f32);
            true
        }
        WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            ..
        } => {
            if *state == ElementState::Released {
                let (x, y) = STATE.lock().unwrap_or_else(|e| e.into_inner()).cursor;
                tap(x, y);
            }
            true
        }
        WindowEvent::Touch(t) => {
            if t.phase == TouchPhase::Ended {
                tap(t.location.x as f32, t.location.y as f32);
            }
            true
        }
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state == ElementState::Pressed {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    || event.logical_key == Key::Named(NamedKey::BrowserBack)
                {
                    cancel();
                } else if event.physical_key == PhysicalKey::Code(KeyCode::Enter) && conflict() {
                    let native = STATE
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .conflict
                        .as_ref()
                        .is_some_and(|c| c.kind == Kind::Mods);
                    resolve(if native {
                        Choice::Cancel
                    } else {
                        Choice::KeepBoth
                    });
                }
            }
            true
        }
        _ => false,
    }
}
