//! A sampling profiler for AL code, for finding what a slow run spends its time on. Off unless the environment
//! variable `AL_PROFILE=<file>` is set. A ticker thread raises a flag every 5 ms; the next statement marker the VM
//! meets records the AL call stack (object and routine of every frame). The file, rewritten every 5 s, ranks the
//! routines by samples where they were the innermost frame ("self") and by samples where they were anywhere on the
//! stack ("inclusive"). One process-wide timer: each tick is taken by the first thread of any kind that reaches a
//! statement marker, so one thread is sampled per tick and a tick no thread takes merges with the next. The file's first
//! line names the interval. A third list ranks the self samples by statement (file and line) and, when the tick fired
//! inside a builtin (FindLast, SetRange, Modify, ...), by that builtin: a sample is also taken when a builtin returns.

use crate::Session;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};

pub(crate) static ON: AtomicBool = AtomicBool::new(false);
static TICK: AtomicBool = AtomicBool::new(false);
/// Microseconds between ticks, for the header.
static PERIOD_US: AtomicU64 = AtomicU64::new(5000);

#[derive(Default)]
struct Counts {
    samples: u64,
    own: HashMap<String, u64>,
    inclusive: HashMap<String, u64>,
    /// Self samples by statement and builtin (`statement_key`).
    statements: HashMap<String, u64>,
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
            PERIOD_US.store(period, Relaxed);
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
    let mut out = format!("{}\n\nself\n", header(c.samples, PERIOD_US.load(Relaxed)));
    for (name, n) in top(&c.own) {
        out.push_str(&format!("{n:8} {name}\n"));
    }
    out.push_str("\ninclusive\n");
    for (name, n) in top(&c.inclusive) {
        out.push_str(&format!("{n:8} {name}\n"));
    }
    out.push_str(
        "\nself by statement (the statement running when the tick fired; [builtin] when it fired inside one)\n",
    );
    for (name, n) in top(&c.statements) {
        out.push_str(&format!("{n:8} {name}\n"));
    }
    let _ = std::fs::write(path, out);
}

/// The first line of the file: the sample count and the unit of one sample.
fn header(samples: u64, period_us: u64) -> String {
    let every = if period_us % 1000 == 0 {
        format!("{} ms", period_us / 1000)
    } else {
        format!("{period_us} us")
    };
    format!("samples {samples} of {every} (one process-wide timer: each tick samples the one thread that takes it)")
}

/// The name of a frame in the file: the object type and name, then the routine. A field trigger carries its field
/// (`table Sales Line.Quantity.OnValidate`; for a table extension the field of the extended table), and a trigger of a
/// page control, action or report data item carries its owner's name.
pub fn frame_name(program: &al_ir::Program, object: usize, routine: usize) -> String {
    let obj = &program.objects[object];
    let Some(r) = obj.routines.get(routine) else {
        return format!("{} {}.?", obj.kind, obj.name);
    };
    let owner = match (r.field, &r.container) {
        (Some(f), _) => {
            let table = match obj.kind {
                al_ir::ObjKind::TableExtension => program
                    .objects
                    .heads()
                    .iter()
                    .find(|o| o.kind == al_ir::ObjKind::Table && o.id.is_some() && o.id == obj.extends_id)
                    .and_then(|o| o.table.as_ref()),
                _ => obj.table.as_ref(),
            };
            table.and_then(|t| t.fields.get(f)).map(|fd| fd.name.clone())
        }
        (None, Some((_, owner))) if r.is_trigger && !owner.is_empty() => Some(owner.clone()),
        _ => None,
    };
    match owner {
        Some(o) => format!("{} {}.{o}.{}", obj.kind, obj.name, r.name),
        None => format!("{} {}.{}", obj.kind, obj.name, r.name),
    }
}

/// The name of a self sample in the "self by statement" list: the innermost frame, the file and line of the statement
/// it was running ("(entry)" before its first statement), and the builtin the sample landed in, if any.
fn statement_key(frame: &str, at: Option<(&str, u32)>, builtin: Option<&str>) -> String {
    let mut k = match at {
        Some((path, line)) => {
            let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
            format!("{frame}  {file}:{line}")
        }
        None => format!("{frame}  (entry)"),
    };
    if let Some(b) = builtin {
        k.push_str(&format!("  [{b}]"));
    }
    k
}

fn top(m: &HashMap<String, u64>) -> Vec<(&String, u64)> {
    let mut v: Vec<(&String, u64)> = m.iter().map(|(k, n)| (k, *n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    v.truncate(60);
    v
}

impl Session {
    /// A builtin returned: takes a pending tick, booked to that builtin. One relaxed load when profiling is off.
    #[inline(always)]
    pub(crate) fn profile_builtin(&self, id: al_builtins::builtin_id::BuiltinId) {
        if ON.load(Relaxed) {
            self.profile_sample(Some(id));
        }
    }

    /// Called when profiling is on: at a statement marker before the frame moves to the new statement (so the sample
    /// names the statement that was running), and after a builtin returns, with its id (so the time a builtin spends
    /// is booked to it and to the statement that called it).
    pub(crate) fn profile_sample(&self, builtin: Option<al_builtins::builtin_id::BuiltinId>) {
        if !TICK.swap(false, Relaxed) {
            return;
        }
        let Some(counts) = COUNTS.get() else { return };
        let program = &self.engine.program;
        let names: Vec<String> = self
            .frames
            .iter()
            .zip(&self.call_stack)
            .map(|(&o, fr)| frame_name(program, o, fr.routine))
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
            let at = self.call_stack.last().and_then(|fr| program.pos(fr.pos)).map(|p| {
                (
                    program.sources.get(p.file as usize).map_or("?", |f| f.path.as_str()),
                    p.line,
                )
            });
            // The overload suffix (`#1`) says nothing to a reader: `Record.FindLast`.
            let key = statement_key(
                last,
                at,
                builtin.map(|b| b.name().split('#').next().unwrap_or_default()),
            );
            *c.statements.entry(key).or_default() += 1;
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

#[cfg(test)]
mod tests {
    #[test]
    fn header_names_the_unit_of_a_sample() {
        assert_eq!(
            super::header(12, 5000),
            "samples 12 of 5 ms (one process-wide timer: each tick samples the one thread that takes it)"
        );
        assert!(super::header(3, 250).starts_with("samples 3 of 250 us "));
    }

    #[test]
    fn statement_key_names_file_line_and_builtin() {
        assert_eq!(
            super::statement_key(
                "codeunit No. Series - Impl..GetNoSeriesLine",
                Some(("src/NoSeries/NoSeriesImpl.Codeunit.al", 132)),
                Some("Record.FindLast")
            ),
            "codeunit No. Series - Impl..GetNoSeriesLine  NoSeriesImpl.Codeunit.al:132  [Record.FindLast]"
        );
        assert_eq!(
            super::statement_key("codeunit X.Run", None, None),
            "codeunit X.Run  (entry)"
        );
        assert_eq!(
            super::statement_key("codeunit X.Run", Some(("a\\b.al", 7)), None),
            "codeunit X.Run  b.al:7"
        );
    }
}
