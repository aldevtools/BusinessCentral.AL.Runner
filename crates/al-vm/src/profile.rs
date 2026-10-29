//! A sampling profiler for AL code, for finding what a slow run spends its time on. Off unless the environment
//! variable `AL_PROFILE=<file>` is set. A ticker thread raises a flag every 5 ms; the next statement marker the VM
//! meets records the AL call stack (object and routine of every frame). The file, rewritten every 5 s, ranks the
//! routines by samples where they were the innermost frame ("self") and by samples where they were anywhere on the
//! stack ("inclusive").

use crate::Session;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};

pub(crate) static ON: AtomicBool = AtomicBool::new(false);
static TICK: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct Counts {
    samples: u64,
    own: HashMap<String, u64>,
    inclusive: HashMap<String, u64>,
}

static COUNTS: OnceLock<Mutex<Counts>> = OnceLock::new();

/// Starts the ticker once, if `AL_PROFILE` is set.
pub(crate) fn init() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let Ok(path) = std::env::var("AL_PROFILE") else { return };
        COUNTS.get_or_init(|| Mutex::new(Counts::default()));
        ON.store(true, Relaxed);
        std::thread::spawn(move || {
            let mut since_write = 0;
            // AL_PROFILE_US: microseconds between samples (default 5000): a short test needs a finer tick.
            let period = std::env::var("AL_PROFILE_US")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000u64);
            let per_write = (5_000_000 / period).max(1);
            loop {
                std::thread::sleep(std::time::Duration::from_micros(period));
                TICK.store(true, Relaxed);
                since_write += 1;
                if since_write >= per_write {
                    since_write = 0;
                    write(&path);
                }
            }
        });
    });
}

/// Writes the file now, if `AL_PROFILE` is set and the profiler ran: the ticker writes it every 5 s only, so a shorter
/// run ends before its first write (issue 6133). The runner calls it when the run ends.
pub fn flush() {
    if !ON.load(Relaxed) {
        return;
    }
    if let Ok(path) = std::env::var("AL_PROFILE") {
        write(&path);
    }
}

fn write(path: &str) {
    let Some(c) = COUNTS.get() else { return };
    let c = c.lock().unwrap();
    let mut out = format!("samples {}\n\nself\n", c.samples);
    for (name, n) in top(&c.own) {
        out.push_str(&format!("{n:8} {name}\n"));
    }
    out.push_str("\ninclusive\n");
    for (name, n) in top(&c.inclusive) {
        out.push_str(&format!("{n:8} {name}\n"));
    }
    let _ = std::fs::write(path, out);
}

fn top(m: &HashMap<String, u64>) -> Vec<(&String, u64)> {
    let mut v: Vec<(&String, u64)> = m.iter().map(|(k, n)| (k, *n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    v.truncate(60);
    v
}

impl Session {
    /// Called at a statement marker when profiling is on.
    pub(crate) fn profile_sample(&self) {
        if !TICK.swap(false, Relaxed) {
            return;
        }
        let Some(counts) = COUNTS.get() else { return };
        let program = &self.engine.program;
        let names: Vec<String> = self
            .frames
            .iter()
            .zip(&self.call_stack)
            .map(|(&o, fr)| {
                let obj = &program.objects[o];
                let r = obj.routines.get(fr.routine).map_or("?", |r| r.name.as_str());
                format!("{} {}.{}", obj.kind, obj.name, r)
            })
            .collect();
        // AL_PROFILE_ONLY=<text>: count only the samples whose stack has a frame containing the text (one test), and
        // write the file as they come, since a short run may end before the periodic write.
        let only = std::env::var("AL_PROFILE_ONLY").ok();
        if let Some(o) = &only {
            if !names.iter().any(|n| n.contains(o.as_str())) {
                return;
            }
        }
        let mut c = counts.lock().unwrap();
        c.samples += 1;
        if let Some(last) = names.last() {
            *c.own.entry(last.clone()).or_default() += 1;
        }
        let mut seen = std::collections::HashSet::new();
        for n in names {
            if seen.insert(n.clone()) {
                *c.inclusive.entry(n).or_default() += 1;
            }
        }
        if only.is_some() && c.samples % 20 == 0 {
            if let Ok(path) = std::env::var("AL_PROFILE") {
                drop(c);
                write(&path);
            }
        }
    }
}
