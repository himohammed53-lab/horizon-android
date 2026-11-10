//! Processor load and heat, logged every 10 seconds: which threads of the game use the CPU,
//! what clock the cores run at, and the battery temperature where the phone lets an app read
//! it. Read-only; it reads a few small files of this process and costs next to nothing.
use std::{collections::HashMap, time::Instant};

const INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);
/// Kernel ticks per second for `/proc/<pid>/stat` times (`sysconf(_SC_CLK_TCK)`, 100 on Android).
const TICKS_PER_SECOND: f64 = 100.0;

/// Starts the sampler once per process.
pub(super) fn start() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let _ = std::thread::Builder::new()
            .name("Load monitor".into())
            .spawn(run);
    });
}

fn run() {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut last: HashMap<String, (String, u64)> = HashMap::new();
    let mut at = Instant::now();
    let mut first = true;
    loop {
        std::thread::sleep(INTERVAL);
        let now = Instant::now();
        let seconds = now.duration_since(at).as_secs_f64().max(0.001);
        at = now;
        let current = threads();
        if !std::mem::take(&mut first) {
            log::info!("{}", line(&last, &current, seconds, cores));
        }
        last = current;
    }
}

/// Thread id to (name, user + system ticks) for every thread of this process.
fn threads() -> HashMap<String, (String, u64)> {
    let mut out = HashMap::new();
    let Ok(dir) = std::fs::read_dir("/proc/self/task") else {
        return out;
    };
    for entry in dir.flatten() {
        let tid = entry.file_name().to_string_lossy().into_owned();
        let Ok(text) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        // "tid (name with spaces) S ppid ... utime stime ...": the name ends at the last ')'.
        let (Some(open), Some(close)) = (text.find('('), text.rfind(')')) else {
            continue;
        };
        let name = text[open + 1..close].to_owned();
        let fields: Vec<&str> = text[close + 1..].split_whitespace().collect();
        // After the name: state is field 0, utime is field 11, stime field 12.
        let ticks = fields
            .get(11)
            .zip(fields.get(12))
            .and_then(|(u, s)| Some(u.parse::<u64>().ok()? + s.parse::<u64>().ok()?));
        if let Some(ticks) = ticks {
            out.insert(tid, (name, ticks));
        }
    }
    out
}

fn line(
    last: &HashMap<String, (String, u64)>,
    current: &HashMap<String, (String, u64)>,
    seconds: f64,
    cores: usize,
) -> String {
    // Threads that ended in between still count for what they used before.
    let mut by_name: HashMap<&str, f64> = HashMap::new();
    let mut total = 0.0;
    for (tid, (name, ticks)) in current {
        let before = last.get(tid).map_or(0, |(_, t)| *t);
        let percent = ticks.saturating_sub(before) as f64 / TICKS_PER_SECOND / seconds * 100.0;
        total += percent;
        *by_name.entry(name.as_str()).or_default() += percent;
    }
    let mut top: Vec<(&str, f64)> = by_name.into_iter().collect();
    top.sort_by(|a, b| b.1.total_cmp(&a.1));
    let top = top
        .iter()
        .take(6)
        .filter(|(_, p)| *p >= 1.0)
        .map(|(name, p)| format!("{name} {p:.0}%"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Horizon: load: cpu {total:.0}% of {}% (100% = one core), threads: {top} | core MHz {} | {}",
        cores * 100,
        clocks(cores),
        battery()
    )
}

/// Each core's current clock in MHz, so a throttled phone shows lower numbers than its maximum.
fn clocks(cores: usize) -> String {
    let speeds: Vec<String> = (0..cores)
        .map(|i| {
            std::fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpu{i}/cpufreq/scaling_cur_freq"
            ))
            .ok()
            .and_then(|t| t.trim().parse::<u64>().ok())
            .map_or("?".to_owned(), |khz| (khz / 1000).to_string())
        })
        .collect();
    format!("[{}]", speeds.join(", "))
}

/// The battery's temperature and charge, when Android lets this app read them.
fn battery() -> String {
    let read = |name: &str| {
        std::fs::read_to_string(format!("/sys/class/power_supply/battery/{name}"))
            .ok()
            .and_then(|t| t.trim().parse::<i64>().ok())
    };
    match (read("temp"), read("capacity")) {
        (Some(temp), Some(level)) => format!("battery {:.1} C, {level}%", temp as f64 / 10.0),
        (Some(temp), None) => format!("battery {:.1} C", temp as f64 / 10.0),
        _ => "battery temperature not readable on this phone".to_owned(),
    }
}
