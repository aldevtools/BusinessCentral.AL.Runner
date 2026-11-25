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
//! - touching everything, each reason with the call stack of its first site: a .NET member that is not kept
//!   ([`dotnet_member_kept`], ADR-079 amendment 1, `dotnet/mod.rs`), a page, a report (`impact.rs`), every builtin
//!   outside [`KEPT_RECEIVERS`] (HTTP, files, StartSession, TaskScheduler and so a Job Queue run, DataTransfer, XMLports),
//!   a table served by a provider other than the app tables (the store), a whole-store read or a company change (the
//!   store).
//!
//! - the row version read by the SystemRowVersion field: the routines the session ran (`install_footprint_row_version.rs`).
//!
//! A write the recorder missed counts as touching everything: at the end the stores are compared with what they held at
//! the start, and every change must be a recorded write (`footprint_check`).

use crate::FaultFrame;
use al_data::memory::{Resource, StoreFootprint};
use std::collections::{BTreeMap, BTreeSet};

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

/// The writes of one session, indexed: what [`Lasting::read_sees_write`] and [`Lasting::writes_collide`] answer against
/// every write, with a lookup per resource (the store's rows through al-data `WriteIndex`).
pub(crate) struct Written<'a> {
    store: al_data::memory::WriteIndex<'a>,
    all: &'a BTreeSet<Lasting>,
}

impl Written<'_> {
    pub(crate) fn any_seen_by(&self, reads: &BTreeSet<Lasting>) -> bool {
        let any_row_write = || {
            self.all
                .iter()
                .any(|w| matches!(w, Lasting::Store(_) | Lasting::RowVersion))
        };
        reads.iter().any(|r| match r {
            Lasting::Store(r) => self.store.seen_by(r),
            Lasting::RowVersion => any_row_write(),
            r => self.all.contains(r),
        })
    }

    pub(crate) fn any_collides_with(&self, writes: &BTreeSet<Lasting>) -> bool {
        writes.iter().any(|w| match w {
            Lasting::Store(w) => self.store.collides_with(w),
            Lasting::RowVersion => false,
            w => self.all.contains(w),
        })
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
    /// Why the session counts as touching everything, each reason with the call stack (outermost first) it was first
    /// noted from; the stack is empty for a reason found at the end of the session (the store's record, the safety net).
    /// Empty when the session does not touch everything.
    pub everything: BTreeMap<String, Vec<FaultFrame>>,
}

impl InstallFootprint {
    /// A footprint that touches everything, for `why`: what an install session whose footprint was not recorded
    /// counts as.
    pub fn touching_everything(why: &str) -> InstallFootprint {
        let mut fp = InstallFootprint::default();
        fp.touch_everything(why);
        fp
    }

    /// Marks the session as touching everything for `why`, found at the end of the session (no site).
    pub fn touch_everything(&mut self, why: impl Into<String>) {
        self.touch_everything_at(why, Vec::new());
    }

    /// Marks the session as touching everything for `why`, noted from `site`; the first site of a reason stays.
    pub fn touch_everything_at(&mut self, why: impl Into<String>, site: Vec<FaultFrame>) {
        self.everything.entry(why.into()).or_insert(site);
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
            || other.written().any_seen_by(&self.reads)
    }

    /// Whether this session and `other` write the same resource, or one of them touches everything.
    pub fn collides(&self, other: &InstallFootprint) -> bool {
        self.touches_everything() || other.touches_everything() || other.written().any_collides_with(&self.writes)
    }

    /// The writes of this session, indexed ([`Written`]).
    pub(crate) fn written(&self) -> Written<'_> {
        let store = self.writes.iter().filter_map(|w| match w {
            Lasting::Store(r) => Some(r),
            _ => None,
        });
        Written {
            store: al_data::memory::WriteIndex::of(store),
            all: &self.writes,
        }
    }

    /// Adds what `other` recorded (the footprint of an app over all its install sessions).
    pub fn extend(&mut self, other: InstallFootprint) {
        self.reads.extend(other.reads);
        self.writes.extend(other.writes);
        self.apps_met.extend(other.apps_met);
        self.apps_all |= other.apps_all;
        for (why, site) in other.everything {
            self.touch_everything_at(why, site);
        }
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
                    self.touch_everything(what);
                }
            }
        }
        for why in s.everything {
            self.touch_everything(why);
        }
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
/// (`Codeunit`: its objects are entered, so its apps are met). `System`, `Database`, `Blob` and `Query` are kept by
/// method ([`KEPT_METHODS`]). Every other builtin counts as touching everything: HTTP, files, pages, reports, XMLports,
/// request pages, TaskScheduler, DataTransfer, Media, CompanyProperty, the test page types and every receiver not named
/// here (a new one included).
const KEPT_RECEIVERS: &[&str] = &[
    "BigInteger",
    "BigText",
    "Boolean",
    "Byte",
    "Codeunit",
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
    "Record",
    "RecordId",
    "RecordRef",
    "SecretText",
    "SessionInformation",
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

/// The methods kept of the receivers kept by method: each reads or writes only values of the session or rows the store
/// records. Every other method of these receivers touches everything (ADR-079 decision 1, "a resource the footprint
/// recorder does not classify"): the encryption key and its eight methods, ImportObjects, ExportObjects,
/// ImportStreamWithUrlAccess, the CodeCoverage methods, Hyperlink, GetDotNetType, CanLoadType; of `Database` the key,
/// data file, license, password, company copy and table connection methods; `Blob` Import and Export; the `Query`
/// SaveAs methods; and every method not named here (a new one included).
const KEPT_METHODS: &[(&str, &[&str])] = &[
    (
        "Blob",
        &["CreateInStream", "CreateOutStream", "HasValue", "Length", "LengthSign"],
    ),
    (
        "Database",
        &[
            "Commit",
            "CompanyName",
            "CurrentTransactionType",
            "IsInWriteTransaction",
            "LastUsedRowVersion",
            "LockTimeout",
            "LockTimeoutDuration",
            "MinimumActiveRowVersion",
            "SelectLatestVersion",
            "SerialNumber",
            "ServiceInstanceId",
            "SessionId",
            "SID",
            "TenantId",
            "UserId",
            "UserSecurityId",
        ],
    ),
    (
        "Query",
        &[
            "Close",
            "ColumnCaption",
            "ColumnName",
            "ColumnNo",
            "GetFilter",
            "GetFilters",
            "Open",
            "Read",
            "SecurityFiltering",
            "SetFilter",
            "SetRange",
            "TopNumberOfRows",
        ],
    ),
    (
        "System",
        &[
            "Abs",
            "ApplicationPath",
            "ArrayLen",
            "AutoFormatWriterGuard",
            "CalcDate",
            "CaptionClassTranslate",
            "Clear",
            "ClearAll",
            "ClearCollectedErrors",
            "ClearLastError",
            "ClearWithEventsTimer",
            "ClosingDate",
            "CompressArray",
            "CopyArray",
            "CopyStream",
            "CreateDateTime",
            "CreateGuid",
            "CurrentDateTime",
            "Date",
            "DaTi",
            "DMY",
            "DT",
            "DWY",
            "Evaluate",
            "EvaluateBounded",
            "EvaluateDateVariable",
            "EvaluateMemberlessVarOption",
            "EvaluateMemberlessVarOptionElement",
            "EvaluateOptionField",
            "EvaluateVariableOption",
            "Format",
            "FormatAutoFormatField",
            "FormatDecimalField",
            "FormatOptionColumn",
            "FormatVariant",
            "GetCollectedErrors",
            "GetDocumentUrl",
            "GetLastErrorCallStack",
            "GetLastErrorCode",
            "GetLastErrorObject",
            "GetLastErrorText",
            "GetUrl",
            "GlobalLanguage",
            "GuiAllowed",
            "HasCollectedErrors",
            "IsCollectingErrors",
            "IsNull",
            "IsNullGuid",
            "IsServiceTier",
            "NormalDate",
            "OptionCaptionML",
            "Power",
            "Random",
            "Randomize",
            "Round",
            "RoundDateTime",
            "Sleep",
            "TemporaryPath",
            "TextConstText",
            "Time",
            "Today",
            "TranslatedText",
            "Variant",
            "WindowsLanguage",
            "WorkDate",
        ],
    ),
];

/// Whether .NET member `member` of type `clr_type` (`None`: a constructor) touches no lasting resource while an install
/// session is recorded (al-runner issue 8954):
/// - one of the pure framework members served in process (al-host `inproc.txt`, ADR-038 section 1). It reads and writes
///   only .NET objects of the session, which live no longer than the install session (ADR-079 fact 1); what install code
///   takes from them reaches a lasting resource only through code the recorder sees;
/// - a server class member whose implementation here answers with a constant or the host identity and keeps nothing
///   (`server_classes::keeps_nothing`); what it answers does not depend on what an install did.
///
/// Every other member touches everything (ADR-079 decision 1), every other server class member included.
fn dotnet_member_kept(clr_type: &str, member: Option<&str>) -> bool {
    al_host::inproc::lists(clr_type, member)
        || member.is_some_and(|m| crate::dotnet::server_classes::keeps_nothing(clr_type, m))
}

/// Whether builtin `name` (`Receiver.Method#n`) touches everything while an install session is recorded
/// ([`KEPT_RECEIVERS`], [`KEPT_METHODS`]). Of `NavApp`, the module info reads are kept (`GetModuleInfo` notes the app it
/// reads); of `Session`, every method but the ones that start, stop or look at another session.
fn builtin_touches_everything(receiver: &str, method: &str) -> bool {
    match receiver {
        "NavApp" => !matches!(method, "GetModuleInfo" | "GetCurrentModuleInfo" | "GetCallerModuleInfo"),
        "Session" => matches!(
            method,
            "StartSession" | "StopSession" | "IsSessionActive" | "IsSessionLoggedIn"
        ),
        r => match KEPT_METHODS.iter().find(|(k, _)| *k == r) {
            Some((_, kept)) => !kept.contains(&method),
            None => !KEPT_RECEIVERS.contains(&r),
        },
    }
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
            fp.touch_everything("writes not committed at the end of the install session");
        } else if let Err((why, table)) = self.store.unrecorded_writes(&b.store, &store_writes) {
            let objects = &self.engine.program.objects;
            let named = table.and_then(|t| objects.get(t as usize)).map(|o| o.name.as_str());
            fp.touch_everything(named.map_or_else(|| why.to_string(), |name| format!("{why} (table {name})")));
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
            fp.touch_everything("a counter, sequence or storage key changed that no recorded write names");
        }
        if self.company_unknown != b.company_unknown || self.full_copies != b.full_copies {
            fp.touch_everything("the companies changed");
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
                fp.touch_everything("the store's record of the install session was lost");
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

    /// The session touches everything for `why`, noted with the call stack of the code that does it (the diagnostics of
    /// a refusal name it, al-runner issue 8954).
    pub(crate) fn note_footprint_everything(&self, why: impl FnOnce() -> String) {
        if !self.coverage.footprint_on {
            return;
        }
        let site = self.stack_now();
        self.note_footprint(|fp| fp.touch_everything_at(why(), site));
    }

    /// A page or a report (`kind`, object index `object`) opens or runs while a footprint is recorded: it touches
    /// everything.
    pub(crate) fn note_footprint_object(&self, kind: &str, object: usize) {
        self.note_footprint_everything(|| format!("a {kind} ({})", self.object_id_text(object)));
    }

    /// A .NET call reaches the host (`dotnet/mod.rs` `dotnet_apply`) while a footprint is recorded: it touches
    /// everything unless its member is kept ([`dotnet_member_kept`]). `shown` names the call.
    pub(crate) fn note_footprint_dotnet(&self, clr_type: &str, member: Option<&str>, shown: &str) {
        if self.coverage.footprint_on && !dotnet_member_kept(clr_type, member) {
            self.note_footprint_everything(|| format!("a .NET call ({shown})"));
        }
    }

    fn object_id_text(&self, object: usize) -> String {
        let o = self.engine.program.objects.get(object);
        o.and_then(|o| o.id)
            .map_or_else(|| "?".to_string(), |id| id.to_string())
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
            let site = if all { self.stack_now() } else { Vec::new() };
            self.note_footprint(|fp| {
                if all {
                    fp.touch_everything_at(format!("builtin {name}"), site);
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
