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
//!   (`ctx.rs` `module_by_id`);
//! - touching everything: .NET, a page, a report (the impact trace), every builtin outside [`KEPT_RECEIVERS`] (HTTP,
//!   files, StartSession, TaskScheduler and so a Job Queue run, DataTransfer, XMLports), a table served by a provider
//!   (the store), a whole-store read or a company change (the store), an impact trace that was already on.
//!
//! - the row version read by the SystemRowVersion field: the routines the session ran (`install_footprint_row_version.rs`).
//!
//! Not yet recorded, so no verdict may rest on a footprint until it counts as a conflict (ADR-079 migration step 1):
//! a write the recorder missed (the safety-net diff of the stores).

use al_data::memory::{Resource, StoreFootprint};
use std::collections::BTreeSet;

/// A lasting resource an install session can read or write.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Why the session counts as touching everything; empty when it does not.
    pub everything: BTreeSet<String>,
}

impl InstallFootprint {
    pub fn touches_everything(&self) -> bool {
        !self.everything.is_empty()
    }

    /// Whether this session reads from `other`, the session of app `other_app`: it met that app, read what `other`
    /// wrote, or one of them touches everything.
    pub fn reads_from(&self, other: &InstallFootprint, other_app: u32) -> bool {
        self.touches_everything()
            || other.touches_everything()
            || self.apps_met.contains(&other_app)
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
        self.everything.extend(other.everything);
    }

    fn add_store(&mut self, s: StoreFootprint) {
        self.reads.extend(s.reads.into_iter().map(Lasting::Store));
        self.writes.extend(s.writes.into_iter().map(Lasting::Store));
        if !s.served.is_empty() {
            self.everything
                .insert(format!("a table served by a provider ({:?})", s.served));
        }
        self.everything.extend(s.everything.into_iter().map(String::from));
    }
}

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

/// The recording in progress (`CoverageState::install_footprint`).
#[derive(Debug, Default)]
pub(crate) struct Recording {
    fp: InstallFootprint,
}

impl crate::Session {
    /// Starts recording the footprint of an install session, replacing one that was recorded.
    pub fn install_footprint_start(&mut self) {
        let mut fp = InstallFootprint::default();
        if self.coverage.impact_on {
            // The objects entered before the recording cannot be told from those entered during it.
            fp.everything.insert("the impact trace was already on".into());
        } else {
            self.enable_impact_trace();
        }
        self.store.footprint_start();
        *self.coverage.install_footprint.borrow_mut() = Some(Box::new(Recording { fp }));
        self.coverage.footprint_on = true;
    }

    /// Stops recording and gives the footprint (`None` when none was recorded).
    pub fn install_footprint_take(&mut self) -> Option<InstallFootprint> {
        self.coverage.footprint_on = false;
        let rec = self.coverage.install_footprint.borrow_mut().take()?;
        let mut fp = rec.fp;
        if let Some(s) = self.store.footprint_take() {
            fp.add_store(s);
        }
        if !fp.everything.contains("the impact trace was already on") {
            let t = self.take_impact_trace();
            self.disable_impact_trace();
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
                fp.everything.insert(format!("a page ({:?})", t.pages));
            }
            if !t.reports.is_empty() {
                fp.everything.insert(format!("a report ({:?})", t.reports));
            }
        }
        Some(fp)
    }

    fn note_footprint(&self, f: impl FnOnce(&mut InstallFootprint)) {
        if let Some(rec) = self.coverage.install_footprint.borrow_mut().as_mut() {
            f(&mut rec.fp);
        }
    }

    /// A builtin call while a footprint is recorded (`exec/run.rs` `run_builtin`): one that touches everything marks it.
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
