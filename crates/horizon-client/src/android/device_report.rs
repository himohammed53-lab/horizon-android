//! One diagnostics block describing the phone and what suits it, logged once per launch.
//! Read-only. The advice is a rule of thumb from memory and core count, not a measurement.
use std::process::Command;

fn property(key: &str) -> String {
    Command::new("getprop")
        .arg(key)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "?".into())
}

/// A `/proc/meminfo` line (`MemTotal`) in MiB.
fn memory_mib(key: &str) -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with(key))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib / 1024)
}

/// Writes the device block to the diagnostics log on its own thread (it starts a few
/// `getprop` processes). `gpu` is the adapter's name, vendor id and driver, `limits` the
/// device's texture and buffer limits.
pub(super) fn log(gpu: String, vendor: u32, limits: Vec<(&'static str, u64)>) {
    let _ = std::thread::Builder::new()
        .name("Device report".into())
        .spawn(move || {
            let cores = std::thread::available_parallelism().map_or(0, |n| n.get());
            let mut speeds: Vec<u64> = (0..cores)
                .filter_map(|i| {
                    std::fs::read_to_string(format!(
                        "/sys/devices/system/cpu/cpu{i}/cpufreq/cpuinfo_max_freq"
                    ))
                    .ok()?
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .map(|khz| khz / 1000)
                })
                .collect();
            speeds.sort_unstable();
            let total = memory_mib("MemTotal").unwrap_or(0);
            let free = memory_mib("MemAvailable").unwrap_or(0);
            log::info!(
                "Horizon: device: {} {} / Android {} (API {}) / chip {} {} {} / {} cores, max MHz {:?} / RAM {} MiB ({} MiB free)",
                property("ro.product.manufacturer"),
                property("ro.product.model"),
                property("ro.build.version.release"),
                property("ro.build.version.sdk"),
                property("ro.soc.manufacturer"),
                property("ro.soc.model"),
                property("ro.board.platform"),
                cores,
                speeds,
                total,
                free,
            );
            log::info!(
                "Horizon: device GPU: {gpu} (vendor 0x{vendor:04x}) limits {}",
                limits
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            // A rule of thumb, not measured: what the memory and cores usually carry, held
            // down by an older or smaller GPU (a Mali-G76 managed about 25 frames per second
            // at the screen's full resolution).
            let (tier, advice) = if let Some(cap) = gpu_cap(&gpu) {
                cap
            } else if total == 0 || cores == 0 {
                ("unknown", "no advice: the phone did not report its memory or cores")
            } else if total < 4096 || cores <= 4 {
                (
                    "low",
                    "render distance 6, resolution scale 50-60%, Battery mode on, 1-2 world workers; try OpenGL ES if Vulkan fails",
                )
            } else if total < 7168 || cores < 8 {
                (
                    "mid",
                    "render distance 8-10, resolution scale 75-100%, default workers",
                )
            } else {
                (
                    "high",
                    "render distance 12 and full resolution are reasonable; lower the distance if it gets hot",
                )
            };
            log::info!("Horizon: device tier: {tier} (rule of thumb, untested): {advice}");
        });
}

/// A tier ceiling from the GPU's name: the memory and cores say little about the graphics.
fn gpu_cap(gpu: &str) -> Option<(&'static str, &'static str)> {
    const LOW: (&str, &str) = (
        "low",
        "render distance 6, resolution scale 50%, Battery mode on, 1-2 world workers; use Vulkan, not OpenGL ES",
    );
    const MID: (&str, &str) = (
        "mid",
        "render distance 8, resolution scale 50-75%; the GPU, not the CPU, sets the frame rate",
    );
    let name = gpu.to_ascii_lowercase();
    let number = |prefix: &str| -> Option<u32> {
        let rest = name.split(prefix).nth(1)?;
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    };
    if name.contains("mali") {
        // Mali-G7x and older mid-range parts (G52, G57, G68, G76) are held to mid or low.
        return match number("-g").or_else(|| number("mali-t")) {
            Some(n) if n < 60 => Some(LOW),
            Some(n) if n < 78 => Some(MID),
            _ => None,
        };
    }
    if name.contains("adreno") {
        return match number("adreno (tm) ").or_else(|| number("adreno ")) {
            Some(n) if n < 610 => Some(LOW),
            Some(n) if n < 650 => Some(MID),
            _ => None,
        };
    }
    name.contains("powervr").then_some(LOW)
}

/// `"low"` or `"mid"` for a GPU that holds the frame rate down at full resolution; `None`
/// for a GPU with no known limit.
pub(crate) fn gpu_tier(gpu: &str) -> Option<&'static str> {
    gpu_cap(gpu).map(|(tier, _)| tier)
}
