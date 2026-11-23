//! The footprint of a partner app's install session (ADR-079 decision 1): every lasting resource the session read or
//! wrote, the apps whose code ran in it or whose presence it read, and why it counts as touching everything. The owner
//! of the footprint type and of the rules which footprints influence each other; the rows part and its conflict rules are
//! al-data `memory/footprint.rs`.
//!
//! Recorded between [`Session::install_footprint_start`] and [`Session::install_footprint_take`] (off otherwise):
//! - rows and table extents: the store (`MemStore::footprint_start`);
//! - number sequences (`sessionops.rs`), isolated storage (`isolated_storage_op`), AutoIncrement counters (`autoinc.rs`),
//!   `Database.LastUsedRowVersion` and `Database.MinimumActiveRowVersion`: one note at each;
//! - apps met: the objects whose code ran (the impact trace's `entered`, `impact.rs`) and `NavApp.GetModuleInfo`
//!   (`ctx.rs` `module_by_id`); a read of an app table ([`APP_TABLES`]) meets every app;
//! - touching everything: .NET, a page, a report (the impact trace), every builtin outside [`KEPT_RECEIVERS`] (HTTP,
//!   files, StartSession, TaskScheduler and so a Job Queue run, DataTransfer, XMLports), a table served by a provider
//!   other than the app tables (the store), a whole-store read or a company change (the store).
//!
//! - the row version read by the SystemRowVersion field: the routines the session ran (`install_footprint_row_version.rs`).
//!
//! A write the recorder missed counts as touching everything: at the end the stores are compared with what they held at
//! the start, and every change must be a recorded write (`footprint_check`).

use al_data::memory::{Resource, StoreFootprint};
use std::collections::BTreeSet;

/// A lasting resource an install session can read or write.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lasting {
    /// A row, a table extent or a whole table (al-data `Resource`).
    Store(Resource),
    /// The AutoIncrement counter of a table: company (empty for `DataPerCompany = false`), table object.
    AutoIncrement(String, u32),
    /// A number sequence: company (empty for a global one), name.
    Sequence(String, String),
    /// An isolated storage key: scope, key.
    IsolatedStorage(String, String),
    /// The database's row version: read by `LastUsedRowVersion` and `MinimumActiveRowVersion`, written by every row
    /// write.
    RowVersion,
}

impl Lasting {
    /// The resource in words, for a baseline note; `table` names a table object.
    pub fn describe(&self, table: impl Fn(u32) -> String) -> String {
        let company = |c: &str| {
            if c.is_empty() {
                String::new()
            } else {
                format!(" in company {c}")
            }
        };
        match self {
            Lasting::Store(Resource::Row(c, t, _)) => format!("a row of table {}{}", table(*t), company(c)),
            Lasting::Store(Resource::Extent(c, t)) => format!("the rows of table {}{}", table(*t), company(c)),
            Lasting::Store(Resource::Table(t)) => format!("table {} in every company", table(*t)),
            Lasting::AutoIncrement(c, t) => format!("the AutoIncrement counter of table {}{}", table(*t), company(c)),
            Lasting::Sequence(c, name) => format!("number sequence {name}{}", company(c)),
            Lasting::IsolatedStorage(scope, key) => format!("isolated storage key {key} ({scope})"),
            Lasting::RowVersion => "the row version of the database".into(),
        }
    }

    /// Whether a read of `self` may see the write `w`.
    pub fn read_sees_write(&self, w: &Lasting) -> bool {
        match (self, w) {
            (Lasting::Store(r), Lasting::Store(w)) => r.read_sees_write(w),
            (Lasting::RowVersion, Lasting::Store(_)) => true,
            (a, b) => a == b,
        }
    }

    /// Whether two writes collide (both write the same row, counter, sequence or storage key).
    pub fn writes_collide(&self, other: &Lasting) -> bool {
        match (self, other) {
            (Lasting::Store(a), Lasting::Store(b)) => a.writes_collide(b),
            (Lasting::RowVersion, _) | (_, Lasting::RowVersion) => false,
            (a, b) => a == b,
        }
    }
}

/// What one install session read and wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallFootprint {
    pub reads: BTreeSet<Lasting>,
    pub writes: BTreeSet<Lasting>,
    /// Apps (indexes into `Program::apps`) whose code ran in the session or whose presence it read.
    pub apps_met: BTreeSet<u32>,
    /// The session read a table that lists the apps (NAV App Installed App and the other app tables): it observes the
    /// presence of every app.
    pub apps_all: bool,
    /// Why the session counts as touching everything; empty when it does not.
    pub everything: BTreeSet<String>,
}

impl InstallFootprint {
    /// A footprint that touches everything, for `why`: what an install session whose footprint was not recorded
    /// counts as.
    pub fn touching_everything(why: &str) -> InstallFootprint {
        InstallFootprint {
            everything: BTreeSet::from([why.to_string()]),
            ..Default::default()
        }
    }

    pub fn touches_everything(&self) -> bool {
        !self.everything.is_empty()
    }

    /// Whether this session met app `app`: code of it ran in the session, or the session read its presence.
    pub fn meets(&self, app: u32) -> bool {
        self.apps_all || self.apps_met.contains(&app)
    }

    /// Whether this session reads from `other`, the session of app `other_app`: it met that app, read what `other`
    /// wrote, or one of them touches everything.
    pub fn reads_from(&self, other: &InstallFootprint, other_app: u32) -> bool {
        self.touches_everything()
            || other.touches_everything()
            || self.meets(other_app)
            || self
                .reads
                .iter()
                .any(|r| other.writes.iter().any(|w| r.read_sees_write(w)))
    }

    /// Whether this session and `other` write the same resource, or one of them touches everything.
    pub fn collides(&self, other: &InstallFootprint) -> bool {
        self.touches_everything()
            || other.touches_everything()
            || self
                .writes
                .iter()
                .any(|a| other.writes.iter().any(|b| a.writes_collide(b)))
    }

    /// Adds what `other` recorded (the footprint of an app over all its install sessions).
    pub fn extend(&mut self, other: InstallFootprint) {
        self.reads.extend(other.reads);
        self.writes.extend(other.writes);
        self.apps_met.extend(other.apps_met);
        self.apps_all |= other.apps_all;
        self.everything.extend(other.everything);
    }

    /// Adds what the store recorded. A table served by a provider (`served`, object indexes of `engine`) is a read of the
    /// presence of every app when it lists the apps ([`APP_TABLES`]), else it touches everything.
    fn add_store(&mut self, s: StoreFootprint, engine: &crate::Engine) {
        self.reads.extend(s.reads.into_iter().map(Lasting::Store));
        self.writes.extend(s.writes.into_iter().map(Lasting::Store));
        for o in s.served {
            let id = engine.program.objects.get(o as usize).and_then(|ob| ob.id);
            match id.and_then(al_builtins::platform::virtual_table) {
                Some(v) if APP_TABLES.contains(&v) => self.apps_all = true,
                _ => {
                    let what = match id {
                        Some(n) => format!("a table served by a provider (table {n})"),
                        None => "a table served by a provider".to_string(),
                    };
                    self.everything.insert(what);
                }
            }
        }
        self.everything.extend(s.everything.into_iter().map(String::from));
    }
}

/// The provider tables whose rows are the apps of the tenant (al-vm `platform/apps.rs`): a read observes which apps are
/// installed, nothing else.
const APP_TABLES: &[al_builtins::platform::Virtual] = &[
    al_builtins::platform::Virtual::NavAppInstalledApp,
    al_builtins::platform::Virtual::NavAppExtra,
    al_builtins::platform::Virtual::InstalledApplication,
    al_builtins::platform::Virtual::PublishedApplication,
];

/// The builtin receivers whose calls touch no lasting resource, or one the recorder notes at its own site: values of the
/// session (text, numbers, collections, JSON, XML, streams, errors), the store's readers and writers (recorded by the
/// store), the isolated storage, number sequence and row version natives (noted in `sessionops.rs`), and code run
/// (`Codeunit`: its objects are entered, so its apps are met). Every other builtin counts as touching everything: HTTP,
/// files, pages, reports, XMLports, request pages, TaskScheduler, DataTransfer, Media, CompanyProperty, the test page
/// types and every receiver not named here (a new one included).
const KEPT_RECEIVERS: &[&str] = &[
    "BigInteger",
    "BigText",
    "Blob",
    "Boolean",
    "Byte",
    "Codeunit",
    "Database",
    "Date",
    "DateTime",
    "Decimal",
    "Dialog",
    "Dictionary",
    "Duration",
    "Enum",
    "ErrorInfo",
    "FieldRef",
    "Guid",
    "HttpContent",
    "HttpHeaders",
    "HttpRequestMessage",
    "HttpResponseMessage",
    "InStream",
    "Integer",
    "Interface",
    "IsolatedStorage",
    "JsonArray",
    "JsonObject",
    "JsonToken",
    "JsonValue",
    "KeyRef",
    "Label",
    "List",
    "ModuleDependencyInfo",
    "ModuleInfo",
    "Notification",
    "NumberSequence",
    "OutStream",
    "ProductName",
    "Query",
    "Record",
    "RecordId",
    "RecordRef",
    "SecretText",
    "SessionInformation",
    "System",
    "Text",
    "TextBuilder",
    "TextConst",
    "Time",
    "Variant",
    "Version",
    "XmlAttribute",
    "XmlAttributeCollection",
    "XmlCData",
    "XmlComment",
    "XmlDeclaration",
    "XmlDocument",
    "XmlDocumentType",
    "XmlElement",
    "XmlNameTable",
    "XmlNamespaceManager",
    "XmlNode",
    "XmlNodeList",
    "XmlProcessingInstruction",
    "XmlReadOptions",
    "XmlText",
    "XmlWriteOptions",
];

/// Whether builtin `name` (`Receiver.Method#n`) touches everything while an install session is recorded
/// ([`KEPT_RECEIVERS`]). Of `NavApp`, the module info reads are kept (`GetModuleInfo` notes the app it reads); of
/// `Session`, every method but the ones that start, stop or look at another session.
fn builtin_touches_everything(receiver: &str, method: &str) -> bool {
    match receiver {
        "NavApp" => !matches!(method, "GetModuleInfo" | "GetCurrentModuleInfo" | "GetCallerModuleInfo"),
        "Session" => matches!(
            method,
            "StartSession" | "StopSession" | "IsSessionActive" | "IsSessionLoggedIn"
        ),
        r => !KEPT_RECEIVERS.contains(&r),
    }
}

/// Object ids of a page or report note, ascending, comma separated.
fn id_list(ids: &std::collections::HashSet<u32>) -> String {
    let mut v: Vec<u32> = ids.iter().copied().collect();
    v.sort_unstable();
    v.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
}

/// The recording in progress (`CoverageState::install_footprint`).
pub(crate) struct Recording {
    fp: InstallFootprint,
    before: Before,
    /// The impact trace that was on when the recording started (the baseline of an impact index build): it goes on
    /// with what the session noted added when the recording stops.
    outer: Option<crate::impact::Touches>,
}

/// What the lasting stores held when the recording started, for the safety net of [`Session::install_footprint_take`]:
/// a change the recorder did not note touches everything.
struct Before {
    store: al_data::memory::MemStore,
    autoinc: std::collections::HashMap<(String, u32), i64>,
    isolated_storage: std::collections::HashMap<(String, String), (String, bool, bool)>,
    sequences: std::collections::HashMap<(String, String), crate::sessionops::Sequence>,
    company_unknown: std::collections::HashMap<(String, u32), String>,
    full_copies: BTreeSet<String>,
}

impl crate::Session {
    fn footprint_before(&self) -> Before {
        Before {
            store: self.store.clone(),
            autoinc: self.autoinc.clone(),
            isolated_storage: self.isolated_storage.clone(),
            sequences: self.sequences.clone(),
            company_unknown: self.company_unknown.clone(),
            full_copies: self.full_copies.clone(),
        }
    }

    /// The safety net: every change of the lasting stores since `b` must be a recorded write of `fp`, else `fp` touches
    /// everything. The write set stays what the sites noted (a write back to the start value shows no change here).
    fn footprint_check(&self, b: &Before, fp: &mut InstallFootprint) {
        let store_writes: BTreeSet<Resource> = (fp.writes.iter())
            .filter_map(|w| match w {
                Lasting::Store(r) => Some(r.clone()),
                _ => None,
            })
            .collect();
        if self.store.has_uncommitted() {
            fp.everything
                .insert("writes not committed at the end of the install session".into());
        } else if let Err(why) = self.store.unrecorded_writes(&b.store, &store_writes) {
            fp.everything.insert(why.into());
        }
        fn changed<K: Eq + std::hash::Hash + Clone, V: PartialEq>(
            a: &std::collections::HashMap<K, V>,
            b: &std::collections::HashMap<K, V>,
        ) -> Vec<K> {
            let gone = b.keys().filter(|k| !a.contains_key(*k));
            (a.iter())
                .filter(|(k, v)| b.get(*k) != Some(*v))
                .map(|(k, _)| k)
                .chain(gone)
                .cloned()
                .collect()
        }
        let mut unnoted = changed(&self.autoinc, &b.autoinc)
            .into_iter()
            .map(|(c, t)| Lasting::AutoIncrement(c, t))
            .chain(
                changed(&self.isolated_storage, &b.isolated_storage)
                    .into_iter()
                    .map(|(a, k)| Lasting::IsolatedStorage(a, k)),
            )
            .chain(
                changed(&self.sequences, &b.sequences)
                    .into_iter()
                    .map(|(c, n)| Lasting::Sequence(c, n)),
            )
            .filter(|l| !fp.writes.contains(l));
        if unnoted.next().is_some() {
            fp.everything
                .insert("a counter, sequence or storage key changed that no recorded write names".into());
        }
        if self.company_unknown != b.company_unknown || self.full_copies != b.full_copies {
            fp.everything.insert("the companies changed".into());
        }
    }
}

impl crate::Session {
    /// Starts recording the footprint of an install session, replacing one that was recorded.
    pub fn install_footprint_start(&mut self) {
        let fp = InstallFootprint::default();
        // The session's own trace is noted apart from one that was on, and added to it when the recording stops.
        let outer = self.coverage.impact_on.then(|| self.take_impact_trace());
        self.enable_impact_trace();
        let before = self.footprint_before();
        self.store.footprint_start();
        *self.coverage.install_footprint.borrow_mut() = Some(Box::new(Recording { fp, before, outer }));
        self.coverage.footprint_on = true;
    }

    /// Stops recording and gives the footprint (`None` when none was recorded).
    pub fn install_footprint_take(&mut self) -> Option<InstallFootprint> {
        self.coverage.footprint_on = false;
        let rec = self.coverage.install_footprint.borrow_mut().take()?;
        let mut fp = rec.fp;
        match self.store.footprint_take() {
            Some(s) => fp.add_store(s, &self.engine),
            None => {
                fp.everything
                    .insert("the store's record of the install session was lost".into());
            }
        }
        self.footprint_check(&rec.before, &mut fp);
        let t = self.take_impact_trace();
        match rec.outer {
            Some(mut outer) => {
                outer.extend(&t);
                *self.coverage.impact.borrow_mut() = Some(outer);
            }
            None => self.disable_impact_trace(),
        }
        let objects = &self.engine.program.objects;
        fp.apps_met
            .extend(t.entered.iter().filter_map(|o| objects.get(*o).map(|o| o.app)));
        let routines = t.routines.iter().copied();
        if crate::install_footprint_row_version::routines_read(&self.engine, routines) {
            fp.reads.insert(Lasting::RowVersion);
        }
        if t.dotnet {
            fp.everything.insert("a .NET call".into());
        }
        if !t.pages.is_empty() {
            fp.everything.insert(format!("a page ({})", id_list(&t.pages)));
        }
        if !t.reports.is_empty() {
            fp.everything.insert(format!("a report ({})", id_list(&t.reports)));
        }
        Some(fp)
    }

    /// The end of a recorded install session: what it left uncommitted is committed, as the end of the session does, so
    /// the safety net compares committed rows. With nothing uncommitted no commit runs, so the transaction flags the
    /// next install reads (`min_row_version.rs`) stay what the plain path leaves.
    pub fn install_footprint_commit(&mut self) {
        if self.store.has_uncommitted() {
            self.commit_transaction();
        }
    }

    fn note_footprint(&self, f: impl FnOnce(&mut InstallFootprint)) {
        if let Some(rec) = self.coverage.install_footprint.borrow_mut().as_mut() {
            f(&mut rec.fp);
        }
    }

    /// A builtin call while a footprint is recorded (`exec/run.rs` `builtin_prologue`, which every builtin path takes): one
    /// that touches everything marks it.
    pub(crate) fn note_footprint_builtin(&self, id: al_builtins::builtin_id::BuiltinId) {
        let name = id.name();
        let (receiver, method) = name.split_once('.').unwrap_or((name, ""));
        let method = method.split('#').next().unwrap_or(method);
        let all = builtin_touches_everything(receiver, method);
        let row_version = crate::install_footprint_row_version::reads_dynamically(receiver, method);
        if all || row_version {
            self.note_footprint(|fp| {
                if all {
                    fp.everything.insert(format!("builtin {name}"));
                }
                if row_version {
                    fp.reads.insert(Lasting::RowVersion);
                }
            });
        }
    }

    /// A number sequence call: `Exists` and `Current` read the sequence, every other method reads and writes it.
    pub(crate) fn note_footprint_sequence(&self, method: &str, key: &(String, String)) {
        let l = Lasting::Sequence(key.0.clone(), key.1.clone());
        self.note_footprint(|fp| {
            fp.reads.insert(l.clone());
            if !matches!(method, "Exists" | "Current") {
                fp.writes.insert(l);
            }
        });
    }

    /// An isolated storage call: `Get` and `Contains` read the key, `Set`, `SetEncrypted` and `Delete` read and write it.
    pub(crate) fn note_footprint_storage(&self, method: &str, slot: &(String, String)) {
        let l = Lasting::IsolatedStorage(slot.0.clone(), slot.1.clone());
        self.note_footprint(|fp| {
            fp.reads.insert(l.clone());
            if !matches!(method, "Get" | "Contains") {
                fp.writes.insert(l);
            }
        });
    }

    /// The AutoIncrement counter of a table was read and set (an Insert that took a number, or a key that raised it).
    pub(crate) fn note_footprint_autoinc(&self, slot: &(String, u32)) {
        let l = Lasting::AutoIncrement(slot.0.clone(), slot.1);
        self.note_footprint(|fp| {
            fp.reads.insert(l.clone());
            fp.writes.insert(l);
        });
    }

    /// The row version of the database was read.
    pub(crate) fn note_footprint_row_version(&self) {
        self.note_footprint(|fp| {
            fp.reads.insert(Lasting::RowVersion);
        });
    }

    /// The presence of app `app` was read (`NavApp.GetModuleInfo`).
    pub(crate) fn note_footprint_app(&self, app: u32) {
        self.note_footprint(|fp| {
            fp.apps_met.insert(app);
        });
    }
}

#[cfg(test)]
#[path = "install_footprint_tests.rs"]
mod tests;
