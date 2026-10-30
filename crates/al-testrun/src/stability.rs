//! Stability testing mode (ADR-058): the config file (`al-runner test --stability FILE|microsoft`), the registry of
//! dimensions with their class (safe, pin first, never), and the variants a config expands to. Pure: the command line
//! reads the file, runs the variants over one compile and reports what changed.
//!
//! A variant is a set of perturbations ([`Perturb`]) the runner applies to the host values and to its own steps
//! around each test. The base variant ([`Perturb::default`]) is a normal run.

use serde_json::Value as Json;

/// The built-in preset `microsoft` (ADR-058 section 3): Microsoft's four perturbations, one at a time.
pub const PRESET_MICROSOFT: &str = r#"{
  "format": 1,
  "dimensions": {
    "random.librarySeed": { "range": [2, 2147483647], "samples": 1 },
    "isolation.alone": [false, true],
    "order.methods": ["declared", "reverse"],
    "workDate.offset": ["+0D", "+1Y"]
  },
  "strategy": "oneAtATime"
}
"#;

/// The text of a preset by name.
pub fn preset(name: &str) -> Option<&'static str> {
    name.eq_ignore_ascii_case("microsoft").then_some(PRESET_MICROSOFT)
}

/// The most variants a `matrix` strategy runs without `--stability-max` (ADR-058 section 3).
pub const DEFAULT_MAX_VARIANTS: usize = 1000;

/// The class of a dimension (ADR-058 section 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Real BC can be in that state and the runtime behaves like BC there.
    Safe,
    /// Real BC can be in that state; the runtime's behavior there is not pinned yet. The text names the missing pin.
    PinFirst(&'static str),
    /// Real BC defines it; changing it makes the runtime differ from BC. The text is the reason.
    Never(&'static str),
}

/// One dimension of the registry: its config name, its class, and whether this build applies it.
#[derive(Debug, Clone, Copy)]
pub struct Dimension {
    pub name: &'static str,
    pub class: Class,
    /// This build of the runner applies it. A safe dimension that is not built yet is refused with that reason.
    pub built: bool,
    /// The value of a normal run (the base variant), as config text.
    pub default: &'static str,
}

const fn dim(name: &'static str, class: Class, built: bool, default: &'static str) -> Dimension {
    Dimension {
        name,
        class,
        built,
        default,
    }
}

/// The registry of dimensions (ADR-058 section 2). A new dimension is a row here and in the ADR's table.
pub const DIMENSIONS: &[Dimension] = &[
    // Microsoft's four first, in the order its stability mode lists them (ADR-058 Context).
    dim("random.librarySeed", Class::Safe, true, "1"),
    dim("isolation.alone", Class::Safe, true, "false"),
    dim("order.methods", Class::Safe, true, "declared"),
    dim("workDate.offset", Class::Safe, true, "+0D"),
    dim("order.codeunits", Class::Safe, true, "declared"),
    dim("random.hostSeed", Class::Safe, true, "1"),
    dim("guid.seed", Class::Safe, true, "1"),
    dim("clock.date", Class::Safe, true, "+0D"),
    dim("clock.timeOfDay", Class::Safe, true, "12:00:00.000"),
    dim("clock.stepMs", Class::Safe, true, "1"),
    dim("identity.userId", Class::Safe, true, "TESTUSER"),
    dim("identity.userSecurityId", Class::Safe, true, ""),
    dim("identity.sessionId", Class::Safe, true, "1"),
    dim("identity.serviceInstanceId", Class::Safe, true, "1"),
    dim("identity.tenantId", Class::Safe, true, "default"),
    dim("company.name", Class::Safe, true, ""),
    dim("numberSeries.offset", Class::Safe, false, "0"),
    dim("data.setup", Class::Safe, false, ""),
    dim(
        "session.timeZone",
        Class::PinFirst(
            "the runtime's time model is UTC; Format and Evaluate of DateTime in other time zones and the list of \
             time zones are not pinned by the corpus (docs/probes-needed.md, row \"Time Zone\")",
        ),
        false,
        "UTC",
    ),
    dim(
        "session.language",
        Class::PinFirst(
            "translated reads are refused (ADR-053, deferred) and Format of numbers and dates in other languages is \
             not pinned by the corpus",
        ),
        false,
        "1033",
    ),
    dim(
        "session.region",
        Class::PinFirst("Format of numbers and dates in other regions is not pinned by the corpus"),
        false,
        "1033",
    ),
    dim(
        "events.subscriberOrder",
        Class::Never(
            "the corpus pins the order of event subscribers across apps as fixed (ascending app id, dependencies \
             first); another order would make the runtime differ from BC",
        ),
        false,
        "",
    ),
    dim(
        "data.syntheticRows",
        Class::Never("AL Runner never writes rows itself: real BC could not reach that state"),
        false,
        "",
    ),
    dim(
        "random.randomizeSequence",
        Class::Never("the sequence after Randomize(n) is defined by BC (types/TestRandomSeededContracts.al)"),
        false,
        "",
    ),
    dim(
        "order.records",
        Class::Never("BC defines the record order on a key"),
        false,
        "",
    ),
    dim(
        "order.triggers",
        Class::Never("BC defines the trigger order"),
        false,
        "",
    ),
    dim("errors.text", Class::Never("BC defines its error texts"), false, ""),
    dim(
        "format.fixedLanguage",
        Class::Never("BC defines Format results for a given language and region"),
        false,
        "",
    ),
    dim("rounding", Class::Never("BC defines rounding"), false, ""),
    dim(
        "autoIncrement.start",
        Class::Never("BC defines AutoIncrement start values"),
        false,
        "",
    ),
    dim(
        "transactions.commit",
        Class::Never("BC defines transaction and commit behavior"),
        false,
        "",
    ),
];

/// The matrix axes of the config (ADR-058 section 3). None is built yet in this version.
pub const MATRIX_AXES: &[&str] = &["bcVersion", "country", "environment", "companies"];

/// The dimension of the registry with this name (exact, as the config spells it).
pub fn dimension(name: &str) -> Option<&'static Dimension> {
    DIMENSIONS.iter().find(|d| d.name == name)
}

/// The order of the test methods inside a codeunit (`order.methods`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MethodOrder {
    #[default]
    Declared,
    Reverse,
    /// Shuffled with this seed.
    Shuffle(u64),
}

impl MethodOrder {
    /// The order of `n` test methods given in declaration order: the indexes into them.
    pub fn arrange(self, n: usize) -> Vec<usize> {
        let mut v: Vec<usize> = (0..n).collect();
        match self {
            MethodOrder::Declared => {}
            MethodOrder::Reverse => v.reverse(),
            MethodOrder::Shuffle(seed) => {
                // Fisher-Yates with splitmix64: the same seed gives the same order on every machine.
                let mut state = seed;
                for i in (1..n).rev() {
                    let j = (splitmix64(&mut state) % (i as u64 + 1)) as usize;
                    v.swap(i, j);
                }
            }
        }
        v
    }
}

/// A change of a date: an offset in years, months and days, or a fixed date (`clock.date`, `workDate.offset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateShift {
    Offset { years: i32, months: i32, days: i32 },
    Fixed { year: i32, month: u32, day: u32 },
}

/// Milliseconds in a day.
const MS_PER_DAY: i64 = 86_400_000;

impl DateShift {
    /// `+1Y`, `-3M`, `+45D`, `+1Y2M3D` (each part signed or following the first sign), or a date `2028-02-29` when
    /// `fixed` is allowed.
    pub fn parse(text: &str, fixed: bool) -> Result<DateShift, String> {
        let t = text.trim();
        if fixed {
            let parts: Vec<&str> = t.split('-').collect();
            if parts.len() == 3
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            {
                let (y, m, d) = (
                    parts[0].parse::<i32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<u32>(),
                );
                if let (Ok(year), Ok(month), Ok(day)) = (y, m, d) {
                    if (1..=9999).contains(&year)
                        && (1..=12).contains(&month)
                        && day >= 1
                        && day <= days_in_month(year, month)
                    {
                        return Ok(DateShift::Fixed { year, month, day });
                    }
                }
                return Err(format!("\"{text}\" is not a valid date"));
            }
        }
        let bad = || {
            if fixed {
                format!("\"{text}\" is neither an offset such as +1Y, -3M or +45D nor a date such as 2028-02-29")
            } else {
                format!("\"{text}\" is not an offset such as +1Y, -3M or +45D")
            }
        };
        let (mut years, mut months, mut days) = (0i32, 0i32, 0i32);
        let mut rest = t;
        let mut sign = 0i32;
        let mut any = false;
        while !rest.is_empty() {
            let (s, r) = match rest.as_bytes()[0] {
                b'+' => (1, &rest[1..]),
                b'-' => (-1, &rest[1..]),
                _ if sign != 0 => (sign, rest),
                _ => return Err(bad()),
            };
            sign = s;
            let digits = r.bytes().take_while(|b| b.is_ascii_digit()).count();
            if digits == 0 || digits >= r.len() {
                return Err(bad());
            }
            let n: i32 = r[..digits].parse().map_err(|_| bad())?;
            let n = n * s;
            match r.as_bytes()[digits].to_ascii_uppercase() {
                b'Y' => years += n,
                b'M' => months += n,
                b'D' => days += n,
                _ => return Err(bad()),
            }
            rest = &r[digits + 1..];
            any = true;
        }
        if !any {
            return Err(bad());
        }
        Ok(DateShift::Offset { years, months, days })
    }

    /// The shift as text, for keys: `2028-02-29`, or the offset as years, months and days (`1Y0M0D`).
    pub fn text(&self) -> String {
        match *self {
            DateShift::Fixed { year, month, day } => format!("{year:04}-{month:02}-{day:02}"),
            DateShift::Offset { years, months, days } => format!("{years}Y{months}M{days}D"),
        }
    }

    /// No change at all.
    pub fn is_zero(&self) -> bool {
        matches!(
            self,
            DateShift::Offset {
                years: 0,
                months: 0,
                days: 0
            }
        )
    }

    /// The AL day number (day 1 = 0001-01-01) `day` moved by this shift. Years and months first, then days; a day of
    /// the month past the end of the target month becomes the month's last day (29 February plus one year is
    /// 28 February).
    pub fn apply_day(&self, day: i32) -> i32 {
        use al_builtins::format::{civil_from_day, day_from_civil};
        match *self {
            DateShift::Fixed { year, month, day } => day_from_civil(year, month, day),
            DateShift::Offset { years, months, days } => {
                let (y, m, d) = civil_from_day(day);
                let total = i64::from(y) * 12 + i64::from(m) - 1 + i64::from(years) * 12 + i64::from(months);
                let (ny, nm) = (total.div_euclid(12) as i32, total.rem_euclid(12) as u32 + 1);
                let nd = d.min(days_in_month(ny, nm));
                day_from_civil(ny, nm, nd) + days
            }
        }
    }

    /// The instant `ms` (Unix milliseconds, UTC) with its date moved by this shift; the time of day stays.
    pub fn apply_ms(&self, ms: i64) -> i64 {
        let epoch = i64::from(al_builtins::format::day_from_civil(1970, 1, 1));
        let unix_day = ms.div_euclid(MS_PER_DAY);
        let time = ms.rem_euclid(MS_PER_DAY);
        let moved = i64::from(self.apply_day((unix_day + epoch) as i32)) - epoch;
        moved * MS_PER_DAY + time
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

/// What a variant changes for the runner (`RunOptions::perturb`). The default is a normal run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Perturb {
    /// `order.methods`.
    pub method_order: MethodOrder,
    /// `isolation.alone`: each test method runs as the only selected test of its codeunit, on a session of its own.
    pub alone: bool,
    /// `random.librarySeed`: after the runner's reset step (codeunit 130301), `Library - Random`.SetSeed(n).
    pub library_seed: Option<i32>,
    /// `workDate.offset`: the work date each test codeunit session starts with, moved.
    pub work_date: Option<DateShift>,
    /// `clock.date`: the date of the run clock, moved or fixed (install baseline and tests).
    pub clock_date: Option<DateShift>,
    /// `order.codeunits`: the order test codeunits run in when they share the database (`--isolation disabled`).
    pub codeunit_order: MethodOrder,
    /// `random.hostSeed`: the seed of the session's `Random` before AL calls `Randomize`.
    pub host_seed: Option<i32>,
    /// `guid.seed`: the seed of `CreateGuid` and new SystemIds.
    pub guid_seed: Option<i32>,
    /// `clock.timeOfDay`: the time of day the run clock starts at, in milliseconds after midnight (UTC).
    pub time_of_day: Option<i64>,
    /// `clock.stepMs`: how far each read of the clock moves it (positive).
    pub step_ms: Option<i64>,
    /// `identity.*`.
    pub identity: al_host::IdentityOverrides,
    /// `company.name`: the session company's name (no test data).
    pub company_name: Option<String>,
}

impl Perturb {
    /// A normal run.
    pub fn is_base(&self) -> bool {
        *self == Perturb::default()
    }

    /// The host values of the variant as text, empty when they are a normal run's: the key of an install baseline,
    /// which install code builds under these values (ADR-058 section 7).
    pub fn host_key(&self) -> String {
        let mut k = Vec::new();
        if let Some(d) = &self.clock_date {
            k.push(format!("date={}", d.text()));
        }
        if let Some(t) = self.time_of_day {
            k.push(format!("tod={t}"));
        }
        if let Some(t) = self.step_ms {
            k.push(format!("step={t}"));
        }
        if let Some(t) = self.host_seed {
            k.push(format!("rnd={t}"));
        }
        if let Some(t) = self.guid_seed {
            k.push(format!("guid={t}"));
        }
        let i = &self.identity;
        if let Some(t) = &i.user_id {
            k.push(format!("user={t}"));
        }
        if let Some(t) = &i.user_security_id {
            k.push(format!("usid={t:?}"));
        }
        if let Some(t) = i.session_id {
            k.push(format!("session={t}"));
        }
        if let Some(t) = i.service_instance_id {
            k.push(format!("instance={t}"));
        }
        if let Some(t) = &i.tenant_id {
            k.push(format!("tenant={t}"));
        }
        if let Some(t) = &self.company_name {
            k.push(format!("company={t}"));
        }
        k.join(";")
    }

    /// The run clock's instant `ms` (Unix milliseconds) under this variant: date moved (`clock.date`), then the time
    /// of day set (`clock.timeOfDay`).
    pub fn clock_ms(&self, ms: i64) -> i64 {
        let ms = self.clock_date.map_or(ms, |d| d.apply_ms(ms));
        match self.time_of_day {
            Some(t) => ms.div_euclid(MS_PER_DAY) * MS_PER_DAY + t,
            None => ms,
        }
    }

    /// Whether the variant changes the clock the sessions read.
    pub fn moves_clock(&self) -> bool {
        self.clock_date.is_some() || self.time_of_day.is_some()
    }
}

/// How the variants are formed (ADR-058 section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strategy {
    #[default]
    OneAtATime,
    Matrix,
    Sample,
}

/// The values of one dimension in the config.
#[derive(Debug, Clone, PartialEq)]
pub enum Values {
    List(Vec<Json>),
    /// A range of integers and how many values to draw from it.
    Range {
        lo: i64,
        hi: i64,
        samples: u32,
    },
}

/// The test selection of a config; the command line's options win over it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Selection {
    pub codeunits: Vec<u32>,
    pub tests: Vec<String>,
    pub exclude_tests: Vec<String>,
}

/// A parsed and checked config.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// In registry order ([`DIMENSIONS`]).
    pub dimensions: Vec<(String, Values)>,
    pub strategy: Strategy,
    pub master_seed: Option<u64>,
    pub runs: u32,
    pub selection: Selection,
}

impl Config {
    /// Whether forming the variants draws values, so the run needs a master seed.
    pub fn draws(&self) -> bool {
        self.strategy == Strategy::Sample
            || self.dimensions.iter().any(|(_, v)| matches!(v, Values::Range { .. }))
            || self.dimensions.iter().any(|(_, v)| match v {
                Values::List(l) => l.iter().any(|j| j.as_str() == Some("shuffle")),
                Values::Range { .. } => false,
            })
    }
}

/// Parses and checks a config text. The error is the reason the run is refused (exit 2, before anything compiles).
pub fn parse_config(text: &str) -> Result<Config, String> {
    let doc: Json = serde_json::from_str(text).map_err(|e| format!("the stability config is not valid JSON: {e}"))?;
    let Some(obj) = doc.as_object() else {
        return Err("the stability config must be a JSON object".into());
    };
    for key in obj.keys() {
        if ![
            "format",
            "matrix",
            "dimensions",
            "strategy",
            "masterSeed",
            "runs",
            "selection",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("the stability config has an unknown field \"{key}\""));
        }
    }
    match obj.get("format").and_then(Json::as_u64) {
        Some(1) => {}
        Some(n) => {
            return Err(format!(
                "the stability config has format {n}; this runner reads format 1"
            ));
        }
        None => return Err("the stability config needs \"format\": 1".into()),
    }
    if let Some(m) = obj.get("matrix") {
        let Some(m) = m.as_object() else {
            return Err("\"matrix\" must be an object".into());
        };
        if let Some(axis) = m.keys().next() {
            if MATRIX_AXES.contains(&axis.as_str()) {
                return Err(format!(
                    "the matrix axis \"{axis}\" is not built yet in this version of the stability mode; use \
                     --bc-version, --country or --environment on the command line for one cell"
                ));
            }
            return Err(format!(
                "\"{axis}\" is not a matrix axis (known: {})",
                MATRIX_AXES.join(", ")
            ));
        }
    }
    let mut dimensions = Vec::new();
    if let Some(d) = obj.get("dimensions") {
        let Some(d) = d.as_object() else {
            return Err("\"dimensions\" must be an object".into());
        };
        for (name, v) in d {
            let Some(reg) = dimension(name) else {
                return Err(format!("\"{name}\" is not a known stability dimension"));
            };
            match reg.class {
                Class::Never(why) => {
                    return Err(format!("the dimension \"{name}\" may never be changed: {why}"));
                }
                Class::PinFirst(missing) => {
                    return Err(format!(
                        "the dimension \"{name}\" needs a corpus pin before it can be changed: {missing}"
                    ));
                }
                Class::Safe if !reg.built => {
                    return Err(format!(
                        "the dimension \"{name}\" is not built yet in this version of the stability mode"
                    ));
                }
                Class::Safe => {}
            }
            let values = parse_values(name, v)?;
            // Each value is checked now, so a bad value is refused before anything compiles.
            match &values {
                Values::List(l) => {
                    for j in l {
                        apply(&mut Perturb::default(), name, j)?;
                    }
                }
                Values::Range { .. } if !RANGE_DIMENSIONS.contains(&name.as_str()) => {
                    return Err(format!("the dimension \"{name}\" takes a list of values, not a range"));
                }
                Values::Range { .. } => {}
            }
            dimensions.push((name.clone(), values));
        }
    }
    // Registry order: the JSON reader does not keep the order of the file, and a variant list must not depend on it.
    dimensions.sort_by_key(|(n, _)| DIMENSIONS.iter().position(|d| d.name == n));
    let strategy = match obj.get("strategy") {
        None => Strategy::OneAtATime,
        Some(s) => match s.as_str() {
            Some("oneAtATime") => Strategy::OneAtATime,
            Some("matrix") => Strategy::Matrix,
            Some("sample") => Strategy::Sample,
            _ => {
                return Err(format!(
                    "\"strategy\" must be \"oneAtATime\", \"matrix\" or \"sample\", not {s}"
                ));
            }
        },
    };
    let master_seed = match obj.get("masterSeed") {
        None => None,
        Some(s) => Some(s.as_u64().ok_or("\"masterSeed\" must be a non-negative integer")?),
    };
    let runs = match obj.get("runs") {
        None => 1,
        Some(r) => match r.as_u64() {
            Some(n) if (1..=10_000).contains(&n) => n as u32,
            _ => return Err("\"runs\" must be an integer from 1 to 10000".into()),
        },
    };
    let mut selection = Selection::default();
    if let Some(s) = obj.get("selection") {
        let Some(s) = s.as_object() else {
            return Err("\"selection\" must be an object".into());
        };
        for (k, v) in s {
            let list = v
                .as_array()
                .ok_or_else(|| format!("\"selection.{k}\" must be a list"))?;
            match k.as_str() {
                "codeunits" => {
                    for c in list {
                        let id = c
                            .as_u64()
                            .and_then(|n| u32::try_from(n).ok())
                            .ok_or("\"selection.codeunits\" holds codeunit ids")?;
                        selection.codeunits.push(id);
                    }
                }
                "tests" | "excludeTests" => {
                    let names = list
                        .iter()
                        .map(|t| t.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| format!("\"selection.{k}\" holds test names"))?;
                    if k == "tests" {
                        selection.tests = names;
                    } else {
                        selection.exclude_tests = names;
                    }
                }
                _ => return Err(format!("\"selection\" has an unknown field \"{k}\"")),
            }
        }
    }
    Ok(Config {
        dimensions,
        strategy,
        master_seed,
        runs,
        selection,
    })
}

fn parse_values(name: &str, v: &Json) -> Result<Values, String> {
    match v {
        Json::Array(l) if !l.is_empty() => Ok(Values::List(l.clone())),
        Json::Array(_) => Err(format!("the dimension \"{name}\" has no values")),
        Json::Object(o) => {
            let range = o.get("range").and_then(Json::as_array);
            let lo = range.and_then(|r| r.first()).and_then(Json::as_i64);
            let hi = range.and_then(|r| r.get(1)).and_then(Json::as_i64);
            let samples = o.get("samples").map_or(Some(1), Json::as_u64);
            match (lo, hi, samples, range.map(Vec::len)) {
                (Some(lo), Some(hi), Some(samples), Some(2)) if lo <= hi && (1..=1000).contains(&samples) => {
                    if o.keys().any(|k| k != "range" && k != "samples") {
                        return Err(format!("the range of \"{name}\" takes only \"range\" and \"samples\""));
                    }
                    Ok(Values::Range {
                        lo,
                        hi,
                        samples: samples as u32,
                    })
                }
                _ => Err(format!(
                    "the range of \"{name}\" must be {{\"range\": [low, high], \"samples\": n}} with low <= high and n from 1 to 1000"
                )),
            }
        }
        _ => Err(format!("the dimension \"{name}\" takes a list of values or a range")),
    }
}

/// The dimensions that take a range of integers.
const RANGE_DIMENSIONS: &[&str] = &[
    "random.librarySeed",
    "random.hostSeed",
    "guid.seed",
    "clock.stepMs",
    "identity.sessionId",
    "identity.serviceInstanceId",
];

/// The dimensions whose printed value is read back as JSON (a number or a boolean), not as text.
const JSON_VALUED: &[&str] = &[
    "isolation.alone",
    "random.librarySeed",
    "random.hostSeed",
    "guid.seed",
    "clock.stepMs",
    "identity.sessionId",
    "identity.serviceInstanceId",
];

/// An order value: `declared`, `reverse`, `shuffle:SEED` or a bare `shuffle` (seed 0, drawn later).
fn order_of(s: &str) -> Option<MethodOrder> {
    match s {
        "declared" => Some(MethodOrder::Declared),
        "reverse" => Some(MethodOrder::Reverse),
        "shuffle" => Some(MethodOrder::Shuffle(0)),
        _ => s
            .strip_prefix("shuffle:")?
            .parse::<u64>()
            .ok()
            .map(MethodOrder::Shuffle),
    }
}

/// `HH:MM:SS` or `HH:MM:SS.mmm`, as milliseconds after midnight.
fn time_of_day(s: &str) -> Option<i64> {
    let (hms, ms) = match s.split_once('.') {
        Some((a, b)) if b.len() == 3 && b.bytes().all(|c| c.is_ascii_digit()) => (a, b.parse::<i64>().ok()?),
        Some(_) => return None,
        None => (s, 0),
    };
    let parts: Vec<&str> = hms.split(':').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.len() != 2 || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let (h, m, sec) = (
        parts[0].parse::<i64>().ok()?,
        parts[1].parse::<i64>().ok()?,
        parts[2].parse::<i64>().ok()?,
    );
    (h < 24 && m < 60 && sec < 60).then_some(((h * 60 + m) * 60 + sec) * 1000 + ms)
}

/// Puts the value `v` of dimension `name` into `p`. Returns the value as text (the form a variant line prints and
/// `--stability-replay` reads back).
pub fn apply(p: &mut Perturb, name: &str, v: &Json) -> Result<String, String> {
    let bad = |what: &str| format!("the dimension \"{name}\" takes {what}, not {v}");
    match name {
        "order.methods" | "order.codeunits" => {
            let s = v
                .as_str()
                .ok_or_else(|| bad("\"declared\", \"reverse\" or \"shuffle:SEED\""))?;
            let order = order_of(s).ok_or_else(|| bad("\"declared\", \"reverse\" or \"shuffle:SEED\""))?;
            if name == "order.methods" {
                p.method_order = order;
            } else {
                p.codeunit_order = order;
            }
            Ok(s.to_string())
        }
        "isolation.alone" => {
            let b = match v {
                Json::Bool(b) => *b,
                Json::String(s) if s == "true" || s == "false" => s == "true",
                _ => return Err(bad("true or false")),
            };
            p.alone = b;
            Ok(b.to_string())
        }
        "random.librarySeed" => {
            let n = match v {
                Json::Number(n) => n.as_i64(),
                Json::String(s) => s.parse::<i64>().ok(),
                _ => None,
            }
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| bad("an integer"))?;
            // Seed 1 is what the runner's reset step sets in a normal run.
            p.library_seed = (n != 1).then_some(n);
            Ok(n.to_string())
        }
        "workDate.offset" | "clock.date" => {
            let s = v.as_str().ok_or_else(|| bad("an offset such as \"+1Y\""))?;
            let shift = DateShift::parse(s, name == "clock.date")?;
            let shift = (!shift.is_zero()).then_some(shift);
            if name == "clock.date" {
                p.clock_date = shift;
            } else {
                p.work_date = shift;
            }
            Ok(s.to_string())
        }
        "random.hostSeed" | "guid.seed" | "identity.sessionId" | "identity.serviceInstanceId" | "clock.stepMs" => {
            let n = match v {
                Json::Number(n) => n.as_i64(),
                Json::String(s) => s.parse::<i64>().ok(),
                _ => None,
            }
            .and_then(|n| i32::try_from(n).ok());
            // The corpus pins positive session and service instance ids (ADR-058 row "identity"); two reads of the
            // clock must differ (ADR-045), so a step is positive.
            let positive = name != "random.hostSeed" && name != "guid.seed";
            let n = match n {
                Some(n) if !positive || n > 0 => n,
                _ if positive => return Err(bad("a positive integer")),
                _ => return Err(bad("an integer")),
            };
            // 1 is the value of a normal run for each of these.
            let set = (n != 1).then_some(n);
            match name {
                "random.hostSeed" => p.host_seed = set,
                "guid.seed" => p.guid_seed = set,
                "identity.sessionId" => p.identity.session_id = set,
                "identity.serviceInstanceId" => p.identity.service_instance_id = set,
                _ => p.step_ms = set.map(i64::from),
            }
            Ok(n.to_string())
        }
        "clock.timeOfDay" => {
            let s = v
                .as_str()
                .ok_or_else(|| bad("a time of day such as \"23:59:59.500\""))?;
            let t = time_of_day(s).ok_or_else(|| bad("a time of day such as \"23:59:59.500\""))?;
            // Noon is the run clock's time of day in a normal run (ADR-045).
            p.time_of_day = (t != MS_PER_DAY / 2).then_some(t);
            Ok(s.to_string())
        }
        "identity.userId" => {
            let s = v.as_str().ok_or_else(|| bad("a user name"))?;
            if s.is_empty() || s.chars().count() > 50 || !s.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(format!(
                    "the dimension \"{name}\" takes a plain name (letters and digits, up to 50 characters), not {v}; \
                     names with other characters (for example \\ or spaces) need a corpus pin first"
                ));
            }
            p.identity.user_id = (s != "TESTUSER").then(|| s.to_string());
            Ok(s.to_string())
        }
        "identity.userSecurityId" => {
            let s = v.as_str().ok_or_else(|| bad("a Guid"))?;
            let g = al_builtins::ops::parse_guid(s)
                .ok_or_else(|| bad("a Guid such as \"{0F1E2D3C-4B5A-6978-8796-A5B4C3D2E1F0}\""))?;
            if g == [0; 16] {
                return Err(bad("a Guid that is not the null Guid"));
            }
            p.identity.user_security_id = Some(g);
            Ok(s.to_string())
        }
        "identity.tenantId" => {
            let s = v.as_str().ok_or_else(|| bad("a tenant id"))?;
            // The corpus pins only that the tenant id is not empty; plain ids keep clear of anything else.
            if s.is_empty() || s.len() > 128 || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Err(bad("a tenant id of letters, digits, - and _ (up to 128 characters)"));
            }
            p.identity.tenant_id = (s != "default").then(|| s.to_string());
            Ok(s.to_string())
        }
        "company.name" => {
            let s = v.as_str().ok_or_else(|| bad("a company name"))?;
            if s.is_empty()
                || s.chars().count() > 30
                || s.starts_with(' ')
                || s.ends_with(' ')
                || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ')
            {
                return Err(format!(
                    "the dimension \"{name}\" takes a name of letters, digits and spaces (up to 30 characters), \
                     not {v}; names with filter characters (& | * ' @) or other characters need a corpus pin first"
                ));
            }
            p.company_name = Some(s.to_string());
            Ok(s.to_string())
        }
        _ => Err(format!("\"{name}\" is not a stability dimension this runner applies")),
    }
}

/// One variant: its number (0 is the base variant), the values it changes, and what that means for the runner.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub index: usize,
    /// (dimension, value as text) of each dimension that differs from its default, in registry order.
    pub values: Vec<(String, String)>,
    pub perturb: Perturb,
}

impl Variant {
    /// `<cell>/<n>`.
    pub fn id(&self, cell: &str) -> String {
        format!("{cell}/{}", self.index)
    }

    /// `order.methods=reverse workDate.offset=+1Y`, or `base`.
    pub fn describe(&self) -> String {
        if self.values.is_empty() {
            return "base".into();
        }
        self.values
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// splitmix64: a small generator that gives the same numbers on every machine.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A generator for draw `k` of run `run` of dimension `dim` under `master`.
fn stream(master: u64, run: u32, dim: usize, k: u64) -> u64 {
    let mut s = master ^ (u64::from(run) << 48) ^ ((dim as u64) << 32) ^ k;
    splitmix64(&mut s);
    s
}

fn draw(lo: i64, hi: i64, state: &mut u64) -> i64 {
    let span = (hi - lo) as u64 + 1;
    lo + (splitmix64(state) % span) as i64
}

/// The concrete values of each dimension for run `run`: lists as given (a bare `shuffle` gets a drawn seed), ranges
/// drawn. The default value is not added.
fn concrete(cfg: &Config, master: u64, run: u32) -> Vec<(String, Vec<Json>)> {
    cfg.dimensions
        .iter()
        .enumerate()
        .map(|(di, (name, values))| {
            let list = match values {
                Values::List(l) => l
                    .iter()
                    .enumerate()
                    .map(|(k, j)| match j.as_str() {
                        Some("shuffle") => {
                            let mut s = stream(master, run, di, k as u64);
                            Json::String(format!("shuffle:{}", splitmix64(&mut s)))
                        }
                        _ => j.clone(),
                    })
                    .collect(),
                Values::Range { lo, hi, samples } => {
                    let mut s = stream(master, run, di, 0);
                    (0..*samples).map(|_| Json::from(draw(*lo, *hi, &mut s))).collect()
                }
            };
            (name.clone(), list)
        })
        .collect()
}

/// Builds the variant for one assignment of values (dimension name, value), base values left out.
fn variant_of(index: usize, assignment: &[(&str, &Json)]) -> Result<Variant, String> {
    let mut perturb = Perturb::default();
    let mut values = Vec::new();
    // Registry order, so a variant line reads the same whatever the order of the file.
    for d in DIMENSIONS {
        if let Some((_, j)) = assignment.iter().find(|(n, _)| *n == d.name) {
            let before = perturb.clone();
            let text = apply(&mut perturb, d.name, j)?;
            if perturb != before {
                values.push((d.name.to_string(), text));
            }
        }
    }
    Ok(Variant { index, values, perturb })
}

/// The variants of a config under `master` (the master seed), the base variant first. `max`: the most variants a
/// `matrix` strategy may form (`--stability-max`).
pub fn variants(cfg: &Config, master: u64, max: usize) -> Result<Vec<Variant>, String> {
    let mut out = vec![Variant {
        index: 0,
        values: Vec::new(),
        perturb: Perturb::default(),
    }];
    let mut seen: Vec<Perturb> = vec![Perturb::default()];
    let mut push = |out: &mut Vec<Variant>, assignment: &[(&str, &Json)]| -> Result<(), String> {
        let v = variant_of(out.len(), assignment)?;
        // A value equal to the default, or a combination already formed, adds no variant.
        if !seen.contains(&v.perturb) {
            seen.push(v.perturb.clone());
            out.push(v);
        }
        Ok(())
    };
    match cfg.strategy {
        Strategy::OneAtATime => {
            for run in 0..cfg.runs {
                for (name, list) in concrete(cfg, master, run) {
                    for j in &list {
                        push(&mut out, &[(name.as_str(), j)])?;
                    }
                }
            }
        }
        Strategy::Matrix => {
            let mut count: usize = 1;
            for run in 0..cfg.runs {
                let dims = concrete(cfg, master, run);
                let per_run = dims
                    .iter()
                    .try_fold(1usize, |acc, (_, l)| acc.checked_mul(l.len() + 1))
                    .unwrap_or(usize::MAX);
                count = count.saturating_add(per_run.saturating_sub(1));
                if count > max {
                    return Err(format!(
                        "the matrix strategy forms {count} or more variants, more than {max}; pass --stability-max N to allow more"
                    ));
                }
                // Every combination, each dimension at its default or one of its values.
                let mut index = vec![0usize; dims.len()];
                loop {
                    let assignment: Vec<(&str, &Json)> = dims
                        .iter()
                        .zip(&index)
                        .filter(|&(_, &k)| k > 0)
                        .map(|((n, l), &k)| (n.as_str(), &l[k - 1]))
                        .collect();
                    if !assignment.is_empty() {
                        push(&mut out, &assignment)?;
                    }
                    let mut i = 0;
                    loop {
                        if i == dims.len() {
                            break;
                        }
                        index[i] += 1;
                        if index[i] <= dims[i].1.len() {
                            break;
                        }
                        index[i] = 0;
                        i += 1;
                    }
                    if i == dims.len() {
                        break;
                    }
                }
            }
        }
        Strategy::Sample => {
            for run in 0..cfg.runs {
                let dims = concrete(cfg, master, run);
                let mut s = stream(master, run, usize::MAX >> 1, 1);
                let picks: Vec<(String, Json)> = dims
                    .iter()
                    .map(|(n, l)| (n.clone(), l[(splitmix64(&mut s) % l.len() as u64) as usize].clone()))
                    .collect();
                let assignment: Vec<(&str, &Json)> = picks.iter().map(|(n, j)| (n.as_str(), j)).collect();
                push(&mut out, &assignment)?;
            }
        }
    }
    Ok(out)
}

/// A variant from its printed values (`--stability-replay "order.methods=reverse workDate.offset=+1Y"`): each
/// `name=value`, separated by spaces. `base` is the base variant.
pub fn parse_values_text(text: &str) -> Result<Variant, String> {
    let t = text.trim();
    if t == "base" {
        return variant_of(0, &[]);
    }
    // A value may hold spaces (`company.name=Other Company`): a word without `=` belongs to the value before it.
    let mut words: Vec<String> = Vec::new();
    for part in t.split_whitespace() {
        match words.last_mut() {
            Some(last) if !part.contains('=') => {
                last.push(' ');
                last.push_str(part);
            }
            _ => words.push(part.to_string()),
        }
    }
    let mut pairs = Vec::new();
    for part in &words {
        let Some((name, value)) = part.split_once('=') else {
            return Err(format!("\"{part}\" is not name=value"));
        };
        let reg = dimension(name).ok_or_else(|| format!("\"{name}\" is not a known stability dimension"))?;
        if !(reg.class == Class::Safe && reg.built) {
            return Err(format!("the dimension \"{name}\" cannot be replayed by this runner"));
        }
        let j = match name {
            n if JSON_VALUED.contains(&n) => serde_json::from_str(value).unwrap_or(Json::from(value)),
            _ => Json::from(value),
        };
        pairs.push((name.to_string(), j));
    }
    let assignment: Vec<(&str, &Json)> = pairs.iter().map(|(n, j)| (n.as_str(), j)).collect();
    variant_of(0, &assignment)
}

/// The variant `--stability-replay` names: an id `<cell>/<n>` (or `<n>`) of the config's variants, else printed
/// values.
pub fn replay(arg: &str, variants: Option<&[Variant]>) -> Result<Variant, String> {
    let number = arg.rsplit('/').next().unwrap_or(arg);
    if let (Ok(n), Some(vs)) = (number.parse::<usize>(), variants) {
        return vs
            .get(n)
            .cloned()
            .ok_or_else(|| format!("the config has {} variants; there is no variant {n}", vs.len()));
    }
    parse_values_text(arg)
}

/// How a test's outcome reads in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Pass,
    Fail,
    NotRun,
}

impl Kind {
    pub fn of(o: &crate::Outcome) -> Kind {
        match o {
            crate::Outcome::Pass => Kind::Pass,
            crate::Outcome::Fail(_) => Kind::Fail,
            crate::Outcome::NotRun(_) => Kind::NotRun,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Kind::Pass => "pass",
            Kind::Fail => "fail",
            Kind::NotRun => "not run",
        }
    }
}

/// A test whose outcome in a variant differs from the base variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// `Codeunit<id>.<test>`.
    pub test: String,
    /// The index of the variant.
    pub variant: usize,
    pub base: Kind,
    pub outcome: Kind,
    /// The message of the variant's outcome (the error or the reason it did not run).
    pub message: String,
    /// The test ran through an unprobed assumption in the variant and not in the base variant.
    pub assumption: bool,
}

impl Change {
    /// A pass that became a failure or a failure that became a pass: a finding. A change to or from "not run" is
    /// listed apart (ADR-058 section 6).
    pub fn is_flip(&self) -> bool {
        self.base != Kind::NotRun && self.outcome != Kind::NotRun
    }
}

/// The outcome of one test in one variant, as the report compares them.
#[derive(Debug, Clone)]
pub struct Seen {
    pub test: String,
    pub kind: Kind,
    pub message: String,
    pub assumption: bool,
}

/// The changes of each variant against the base variant (`per_variant[0]`). A test missing from a variant is not
/// compared.
pub fn changes(per_variant: &[Vec<Seen>]) -> Vec<Change> {
    let Some(base) = per_variant.first() else {
        return Vec::new();
    };
    let base: std::collections::HashMap<&str, &Seen> = base.iter().map(|s| (s.test.as_str(), s)).collect();
    let mut out = Vec::new();
    for (vi, seen) in per_variant.iter().enumerate().skip(1) {
        for s in seen {
            let Some(b) = base.get(s.test.as_str()) else { continue };
            if b.kind != s.kind {
                out.push(Change {
                    test: s.test.clone(),
                    variant: vi,
                    base: b.kind,
                    outcome: s.kind,
                    message: s.message.clone(),
                    assumption: s.assumption && !b.assumption,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(text: &str) -> Config {
        parse_config(text).unwrap_or_else(|e| panic!("{e}"))
    }

    fn refused(text: &str) -> String {
        parse_config(text).expect_err("refused")
    }

    #[test]
    fn the_microsoft_preset_expands_to_five_variants() {
        let c = cfg(PRESET_MICROSOFT);
        let vs = variants(&c, 4711, DEFAULT_MAX_VARIANTS).unwrap();
        assert_eq!(vs.len(), 5, "{vs:#?}");
        assert!(vs[0].perturb.is_base());
        assert_eq!(vs[0].describe(), "base");
        let lines: Vec<String> = vs[1..].iter().map(Variant::describe).collect();
        assert!(lines[0].starts_with("random.librarySeed="), "{lines:?}");
        assert_eq!(
            lines[1..],
            ["isolation.alone=true", "order.methods=reverse", "workDate.offset=+1Y"]
        );
        let seed = vs[1].perturb.library_seed.unwrap();
        assert!(seed >= 2);
        // The same master seed gives the same seed; another gives another.
        assert_eq!(variants(&c, 4711, 1000).unwrap()[1], vs[1]);
        assert_ne!(variants(&c, 4712, 1000).unwrap()[1].perturb.library_seed, Some(seed));
        assert_eq!(vs[4].id("29.0-w1"), "29.0-w1/4");
    }

    #[test]
    fn never_dimensions_are_refused_with_their_reason() {
        for d in DIMENSIONS {
            if let Class::Never(why) = d.class {
                let e = refused(&format!(r#"{{"format":1,"dimensions":{{"{}":["x"]}}}}"#, d.name));
                assert!(e.contains("may never be changed") && e.contains(why), "{e}");
            }
        }
        let e = refused(r#"{"format":1,"dimensions":{"events.subscriberOrder":["reverse"]}}"#);
        assert!(e.contains("ascending app id"), "{e}");
    }

    #[test]
    fn pin_first_dimensions_are_refused_with_the_missing_pin() {
        let mut n = 0;
        for d in DIMENSIONS {
            if let Class::PinFirst(missing) = d.class {
                let e = refused(&format!(r#"{{"format":1,"dimensions":{{"{}":["x"]}}}}"#, d.name));
                assert!(e.contains("needs a corpus pin") && e.contains(missing), "{e}");
                n += 1;
            }
        }
        assert_eq!(n, 3);
    }

    #[test]
    fn unknown_names_unbuilt_dimensions_and_matrix_axes_are_refused() {
        assert!(refused(r#"{"format":1,"dimensions":{"clock.weather":["x"]}}"#).contains("not a known"));
        assert!(refused(r#"{"format":1,"dimensions":{"numberSeries.offset":[2]}}"#).contains("not built yet"));
        assert!(refused(r#"{"format":1,"matrix":{"bcVersion":["29.0"]}}"#).contains("not built yet"));
        assert!(refused(r#"{"format":1,"matrix":{"planet":["x"]}}"#).contains("not a matrix axis"));
        assert!(refused(r#"{"format":1,"colour":1}"#).contains("unknown field"));
        assert!(refused(r#"{"format":2}"#).contains("format 2"));
        assert!(refused(r#"{"dimensions":{}}"#).contains("format"));
        assert!(refused("not json").contains("not valid JSON"));
    }

    #[test]
    fn bad_values_are_refused_before_anything_runs() {
        assert!(refused(r#"{"format":1,"dimensions":{"order.methods":["sideways"]}}"#).contains("order.methods"));
        assert!(refused(r#"{"format":1,"dimensions":{"workDate.offset":["+1W"]}}"#).contains("+1W"));
        assert!(refused(r#"{"format":1,"dimensions":{"workDate.offset":["2028-02-29"]}}"#).contains("2028-02-29"));
        assert!(refused(r#"{"format":1,"dimensions":{"clock.date":["2027-02-29"]}}"#).contains("not a valid date"));
        assert!(refused(r#"{"format":1,"dimensions":{"isolation.alone":[1]}}"#).contains("true or false"));
        assert!(refused(r#"{"format":1,"dimensions":{"isolation.alone":{"range":[0,1]}}}"#).contains("list"));
        assert!(refused(r#"{"format":1,"dimensions":{"random.librarySeed":{"range":[5,1]}}}"#).contains("low <= high"));
        assert!(refused(r#"{"format":1,"strategy":"all"}"#).contains("strategy"));
    }

    #[test]
    fn one_at_a_time_forms_one_variant_per_non_default_value() {
        let c = cfg(
            r#"{"format":1,"dimensions":{"order.methods":["declared","reverse","shuffle:7"],"clock.date":["+0D","2028-02-29","-3M"]}}"#,
        );
        let vs = variants(&c, 1, 1000).unwrap();
        let lines: Vec<String> = vs.iter().map(Variant::describe).collect();
        assert_eq!(
            lines,
            [
                "base",
                "order.methods=reverse",
                "order.methods=shuffle:7",
                "clock.date=2028-02-29",
                "clock.date=-3M"
            ]
        );
        assert_eq!(vs[2].perturb.method_order, MethodOrder::Shuffle(7));
    }

    #[test]
    fn matrix_forms_every_combination_and_is_capped() {
        let text = r#"{"format":1,"strategy":"matrix","dimensions":{"order.methods":["reverse"],"isolation.alone":[true],"workDate.offset":["+1Y","-1D"]}}"#;
        let vs = variants(&cfg(text), 1, 1000).unwrap();
        // 2 * 2 * 3 combinations.
        assert_eq!(vs.len(), 12);
        assert!(
            vs.iter()
                .any(|v| v.describe() == "isolation.alone=true order.methods=reverse workDate.offset=-1D")
        );
        let e = variants(&cfg(text), 1, 5).unwrap_err();
        assert!(e.contains("--stability-max"), "{e}");
    }

    #[test]
    fn sample_draws_runs_variants_from_the_master_seed() {
        let text = r#"{"format":1,"strategy":"sample","runs":20,"dimensions":{"random.librarySeed":{"range":[2,1000000],"samples":5},"order.methods":["declared","reverse"]}}"#;
        let a = variants(&cfg(text), 99, 1000).unwrap();
        let b = variants(&cfg(text), 99, 1000).unwrap();
        assert_eq!(a, b);
        assert!(a.len() > 2 && a.len() <= 21);
        assert!(cfg(text).draws());
        assert!(!cfg(r#"{"format":1,"dimensions":{"order.methods":["reverse"]}}"#).draws());
    }

    #[test]
    fn a_variant_is_replayed_from_its_id_or_its_printed_values() {
        let c = cfg(PRESET_MICROSOFT);
        let vs = variants(&c, 4711, 1000).unwrap();
        assert_eq!(replay("29.0-w1/3", Some(&vs)).unwrap(), vs[3]);
        assert_eq!(replay("3", Some(&vs)).unwrap(), vs[3]);
        assert!(replay("29.0-w1/9", Some(&vs)).unwrap_err().contains("no variant 9"));
        let from_text = replay(&vs[1].describe(), None).unwrap();
        assert_eq!(from_text.perturb, vs[1].perturb);
        let both = replay("order.methods=reverse workDate.offset=+1Y", None).unwrap();
        assert_eq!(both.perturb.method_order, MethodOrder::Reverse);
        assert!(both.perturb.work_date.is_some());
        assert!(replay("base", None).unwrap().perturb.is_base());
        assert!(replay("numberSeries.offset=3", None).is_err());
        // Phase 2: a value with spaces and numbers read back.
        let co = replay("company.name=Other Company clock.stepMs=5", None).unwrap();
        assert_eq!(co.perturb.company_name.as_deref(), Some("Other Company"));
        assert_eq!(co.perturb.step_ms, Some(5));
        assert_eq!(replay(&co.describe(), None).unwrap().perturb, co.perturb);
    }

    #[test]
    fn phase_two_values_are_checked_before_anything_runs() {
        let one = |name: &str, v: &str| parse_config(&format!(r#"{{"format":1,"dimensions":{{"{name}":[{v}]}}}}"#));
        // Two reads of the clock must differ (ADR-045).
        assert!(one("clock.stepMs", "0").unwrap_err().contains("positive"));
        assert!(one("identity.sessionId", "-1").unwrap_err().contains("positive"));
        assert!(
            one("identity.userId", r#""DOMAIN.user""#)
                .unwrap_err()
                .contains("pin first")
        );
        assert!(
            one("identity.userId", r#""two words""#)
                .unwrap_err()
                .contains("pin first")
        );
        assert!(one("company.name", r#""A&B""#).unwrap_err().contains("pin first"));
        assert!(one("company.name", r#""a name longer than thirty chars""#).is_err());
        assert!(one("identity.tenantId", r#""""#).is_err());
        assert!(one("identity.userSecurityId", r#""{00000000-0000-0000-0000-000000000000}""#).is_err());
        assert!(one("clock.timeOfDay", r#""24:00:00""#).is_err());
        assert!(one("clock.timeOfDay", r#""23:59:59.5""#).is_err());
        assert!(one("clock.timeOfDay", r#""23:59:59.500""#).is_ok());
        assert!(one("order.codeunits", r#""reverse""#).is_ok());
        // The integer dimensions take a range.
        let c = cfg(r#"{"format":1,"dimensions":{"random.hostSeed":{"range":[2,1000],"samples":2}},"masterSeed":3}"#);
        let vs = variants(&c, 3, 1000).unwrap();
        assert!(
            vs[1..]
                .iter()
                .all(|v| v.perturb.host_seed.is_some_and(|n| (2..=1000).contains(&n)))
        );
        assert!(refused(r#"{"format":1,"dimensions":{"company.name":{"range":[1,2]}}}"#).contains("list"));
        // Values equal to a normal run's form no variant.
        let c = cfg(r#"{"format":1,"dimensions":{"clock.timeOfDay":["12:00:00.000"],"identity.userId":["TESTUSER"]}}"#);
        assert_eq!(variants(&c, 0, 1000).unwrap().len(), 1);
    }

    #[test]
    fn date_shifts_move_dates_by_calendar_rules() {
        use al_builtins::format::day_from_civil;
        let s = DateShift::parse("+1Y", false).unwrap();
        assert_eq!(s.apply_day(day_from_civil(2024, 2, 29)), day_from_civil(2025, 2, 28));
        assert_eq!(
            DateShift::parse("-3M", false)
                .unwrap()
                .apply_day(day_from_civil(2026, 1, 31)),
            day_from_civil(2025, 10, 31)
        );
        assert_eq!(
            DateShift::parse("+45D", false)
                .unwrap()
                .apply_day(day_from_civil(2026, 10, 30)),
            day_from_civil(2026, 12, 14)
        );
        assert_eq!(
            DateShift::parse("+1Y2M", false)
                .unwrap()
                .apply_day(day_from_civil(2026, 10, 30)),
            day_from_civil(2027, 12, 30)
        );
        assert!(DateShift::parse("+0D", false).unwrap().is_zero());
        // An instant keeps its time of day.
        let noon_2026_10_30 = 1_793_361_600_000i64;
        let moved = DateShift::parse("2028-02-29", true).unwrap().apply_ms(noon_2026_10_30);
        assert_eq!(moved.rem_euclid(MS_PER_DAY), MS_PER_DAY / 2);
        assert_eq!(
            DateShift::parse("+1Y", true).unwrap().apply_ms(noon_2026_10_30) - noon_2026_10_30,
            365 * MS_PER_DAY
        );
    }

    #[test]
    fn method_orders_arrange_indexes() {
        assert_eq!(MethodOrder::Declared.arrange(3), [0, 1, 2]);
        assert_eq!(MethodOrder::Reverse.arrange(3), [2, 1, 0]);
        let s = MethodOrder::Shuffle(42).arrange(10);
        assert_eq!(s, MethodOrder::Shuffle(42).arrange(10));
        let mut sorted = s.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn changes_compare_each_variant_with_the_base() {
        let seen = |t: &str, k: Kind, a: bool| Seen {
            test: t.into(),
            kind: k,
            message: String::new(),
            assumption: a,
        };
        let runs = vec![
            vec![
                seen("A", Kind::Pass, false),
                seen("B", Kind::Pass, false),
                seen("C", Kind::Fail, false),
            ],
            vec![
                seen("A", Kind::Fail, false),
                seen("B", Kind::NotRun, false),
                seen("C", Kind::Fail, false),
            ],
            vec![
                seen("A", Kind::Pass, false),
                seen("B", Kind::Pass, false),
                seen("C", Kind::Pass, true),
            ],
        ];
        let c = changes(&runs);
        assert_eq!(c.len(), 3);
        assert!(c[0].is_flip() && c[0].test == "A" && c[0].variant == 1);
        assert!(!c[1].is_flip(), "pass -> not run is listed apart");
        assert!(c[2].is_flip() && c[2].assumption);
    }
}
